use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicU32}};

use tempdir::TempDir;
use tokio::sync::{Mutex, OwnedMutexGuard, OwnedSemaphorePermit, RwLock, Semaphore};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{sdebug, sinfo};
use trafficstar_logger_macro::StructLoggerName;

use crate::{async_run_command, get_singleton_multi};
use once_cell::sync::OnceCell;

#[derive(StructLoggerName)]
pub struct DirectoryCopyHolder{
    entries : Arc<RwLock<HashMap<PathBuf,Arc<CopyHandler>>>>
}

static INSTANCE: OnceCell<DirectoryCopyHolder> = OnceCell::new();

impl DirectoryCopyHolder{

    fn handler() -> &'static DirectoryCopyHolder {
        INSTANCE.get_or_init(|| {
            DirectoryCopyHolder {
                entries: Arc::new(RwLock::new(HashMap::new()))
            }
        })
    }

    async fn make_handler(path : PathBuf) -> Arc<CopyHandler>{
        let mut handler = Self::handler().entries.write().await;
        handler.insert(path.clone(), Arc::new(CopyHandler::new(path.clone())));
        sdebug!("Made new CopyHandler for directory {:?}",path);
        handler.get(&path).unwrap().clone()
    }

    pub async fn get_handler(path : &Path) -> Arc<CopyHandler>{
        let handler = Self::handler().entries.read().await;
        
        if let Some(entry) = handler.get(path){
            entry.clone()
        }else{
            drop(handler);
            Self::make_handler(path.to_path_buf()).await
        }
    }
}


#[derive(StructLoggerName)]
pub struct CopyHandler{
    original_path : PathBuf,
    free_copies : Arc<Mutex<Vec<Arc<TempDir>>>>,
    sempahore : Arc<Semaphore>,
    total_copies : Arc<AtomicU32>
}

impl CopyHandler{

     async fn copy_files(source_path: &Path, target_path: &Path) -> Result<(), TrafficStarError> {
        // Ensure the target directory exists
        if !target_path.exists() {
           return Err(format!("Target directory {:?} dont exists!",target_path).into())
        }

        let source_path = source_path.to_path_buf().join(".");


        let Some(source_path_str) = source_path.to_str() else { return Err("Couldn't convert path into a string!".into()) };
        
        let Some(target_path_str) = target_path.to_str() else { return Err("Couldn't convert path into a string!".into()) };
        
        match async_run_command("cp", vec!["-r",source_path_str,target_path_str]).await?.status.success(){
            true => Ok(()),
            false => {
                Err("Bad exit status!".into())
            },
        }
         
    
        
    }

    fn new(original_path : PathBuf) -> Self{
        Self{
            original_path, 
            free_copies : Arc::new(Mutex::new(Vec::new())),
            sempahore : Arc::new(Semaphore::new(0)),
            total_copies : Arc::new(AtomicU32::new(0))
        }
    }

    async fn create_copy(&self, copies : OwnedMutexGuard<Vec<Arc<TempDir>>>) -> Result<CopyHandlerHolder, TrafficStarError>{
        let new_tempdir = TempDir::new("CopyHandler")?;
        let tempdir_path = new_tempdir.path().to_path_buf();
        let entry = Arc::new(new_tempdir);
        
        Self::copy_files(&self.original_path, &tempdir_path).await?;
        
        
        

        drop(copies);
        Ok(CopyHandlerHolder{
            path: tempdir_path,
            permit: None,
            entry,
            free_copies : self.free_copies.clone(),
            semaphore : self.sempahore.clone()
        })
    }

    async fn get_copy_inner(&self, max_copies : u32, mut permit : Option<OwnedSemaphorePermit>) -> Result<CopyHandlerHolder,TrafficStarError>{
        sinfo!("Permits : {}",self.sempahore.available_permits());
        loop{
            match permit{
                Some(v) => {
                        let mut inner = self.free_copies.lock().await;
                        if let Some(copy) = inner.pop(){

                            return Ok(CopyHandlerHolder{
                                path: copy.path().to_path_buf(),
                                permit: Some(Arc::new(v)),
                                entry: copy,
                                free_copies: self.free_copies.clone(),
                                semaphore : self.sempahore.clone()
                            })
                        }else{
                            return Err(TrafficStarError::msg("Held semaphore but free_copies lacked a free directory!".into()))
                        }
                    
                },
                None => {
                    match self.sempahore.clone().try_acquire_owned(){
                        Ok(v) => {
                            permit = Some(v);
                            continue;
                        },
                        Err(_) => {
                            loop{
                                let copies = self.total_copies.load(std::sync::atomic::Ordering::Acquire);
                                if copies >= max_copies{
                                    permit = match self.sempahore.clone().acquire_owned().await{
                                        Ok(v) => {
                                            Some(v)
                                        },
                                        Err(err) => return Err(format!("Couldn't acquire permit, error : {}",err).into()),
                                    };
                                    break;
                                }
                                else{
                                        let lock: OwnedMutexGuard<Vec<Arc<TempDir>>> = self.free_copies.clone().lock_owned().await;
                                        if self.total_copies.fetch_add(1,std::sync::atomic::Ordering::AcqRel) < max_copies{
                                            return self.create_copy(lock).await
                                        }else{
                                            self.total_copies.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
                                        }
                                        drop(lock);
                                }
                            }
                        },
                    }
                },
            }
        }
    }

    ///Default max_copies is u32::MAX
    pub async fn get_copy(&self, max_copies : Option<u32>) -> Result<CopyHandlerHolder,TrafficStarError>{
        
        self.get_copy_inner(max_copies.unwrap_or(u32::MAX), None).await
    }
}

#[derive(StructLoggerName)]
pub struct CopyHandlerHolder{
    path : PathBuf,
    permit : Option<Arc<OwnedSemaphorePermit>>,
    entry : Arc<TempDir>,
    free_copies : Arc<Mutex<Vec<Arc<TempDir>>>>,
    semaphore : Arc<Semaphore>
}

impl CopyHandlerHolder{
    pub fn get_path(&self) -> &Path{
        &self.path
    }
}

impl Drop for CopyHandlerHolder{
    fn drop(&mut self) {
        let permit = self.permit.clone();
        let entry = self.entry.clone();
        let free_copies = self.free_copies.clone();
        let semaphore = self.semaphore.clone();
        get_singleton_multi().spawn(async move {
          
            sdebug!("Locking CopyHandlerHolderEntry!");
            free_copies.lock().await.push(entry);
            sdebug!("Locked CopyHandlerHolderEntry!");
            if let Some(permit) = permit{
                drop(permit);
            }else{
                semaphore.add_permits(1);
            }
        });
        
        
    }
}