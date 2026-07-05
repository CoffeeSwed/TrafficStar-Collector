use std::{collections::HashMap, mem::MaybeUninit, path::PathBuf, sync::{Arc, Once}};

use tempdir::TempDir;
use tokio::{fs::File, sync::RwLock};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use uuid::Uuid;
pub mod directory_copies;
#[cfg(test)]
pub mod test;

pub struct TempFileHandler{
    directories : Arc<RwLock<HashMap<String,TempDir>>>   
}

impl TempFileHandler{
    #[allow(unsafe_code, static_mut_refs)]
    pub fn get_singleton() -> Arc<Self>{
        static mut SINGLETON: MaybeUninit<Arc<TempFileHandler>> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        // SAFETY:
        // Needed to create a singleton of an initializable as a static.
        unsafe {
            ONCE.call_once(|| {
                SINGLETON.write(Arc::new(TempFileHandler{
                    directories : Arc::new(RwLock::new(HashMap::new()))
                }));
            });

            SINGLETON.assume_init_mut().clone()
        }
    }


    pub async fn get_directory(directory_name : &str) -> Result<PathBuf, TrafficStarError>{
        let singleton = Self::get_singleton();
        let mut lock = singleton.directories.write().await;
        if let Some(item) = lock.get(directory_name){
            Ok(item.path().into())
        }else{
            let tempdir = match TempDir::new(directory_name){
                Ok(v) => v,
                Err(err) => return Err(format!("Could not create tempdir, reason : {}",err).into()),
            };
            let path = tempdir.path().to_path_buf();
            lock.insert(directory_name.to_string(), tempdir);
            Ok(path)
        }
    }

    pub async fn get_tempfile(directory_name : &str) -> Result<PathBuf, TrafficStarError>{
        let tempdir = Self::get_directory(directory_name).await?;
        loop{
            let file = tempdir.join(Uuid::new_v4().to_string());
            match File::create_new(file.as_path()).await{
                Ok(v) => {
                    drop(v);
                    return Ok(file)
                },
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::AlreadyExists{
                        continue;
                    }else{
                        return Err(err.into())
                    }
                },
            }
        }
    }
    
}