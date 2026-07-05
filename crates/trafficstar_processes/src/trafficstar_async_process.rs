use std::{process::{ExitStatus, Stdio}, sync::{Arc}};

use futures::{AsyncReadExt};
use nix::libc::pid_t;
use tokio::{io::AsyncWriteExt as _, process::{Child, ChildStdin, Command}, sync::RwLock};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::sdebug;
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{get_singleton_multi, trafficstar_files::file_handler::FileHandler};



#[derive(StructLoggerName, Clone)]
pub struct ASyncProcess {
    pub handle: Arc<RwLock<Child>>,
    stdout : Arc<RwLock<FileHandler>>,
    stdin : Arc<RwLock<ChildStdin>>,
    stderr : Arc<RwLock<FileHandler>>,
    pub name : String,
}

#[derive(strum_macros::Display, PartialEq)]
pub enum AsyncProcessReadFrom{
    Stdout,
    Stderr
}

impl ASyncProcess {
    pub async fn new(
        mut command : Command, name : String) -> Result<Arc<ASyncProcess>, TrafficStarError>{
        command.stdin(Stdio::piped());
        command.stderr(Stdio::piped());
        command.stdout(Stdio::piped());
        let mut handle = command.spawn()?;
        let stdout = Arc::new(RwLock::new(
            FileHandler::new(handle.stdout.take().unwrap().into_owned_fd()?).await?
        ));
        let stdin = Arc::new(RwLock::new(
            handle.stdin.take().unwrap())
        );
        let stderr = Arc::new(RwLock::new(

            FileHandler::new(handle.stderr.take().unwrap().into_owned_fd()?).await?
        ));
        
        

        Ok(ASyncProcess {
            handle: Arc::new(RwLock::new(handle)),
            name,
            stderr,
            stdin,
            stdout,
        }.into())
    }
    
    pub async fn read_line(&self, from : AsyncProcessReadFrom) -> Result<String, TrafficStarError>{
        match from{
            AsyncProcessReadFrom::Stdout => {
                let mut lock = self.stdout.write().await;
                lock.read_string(Vec::with_capacity(4096),Some(b'\n')).await
                
            },
            AsyncProcessReadFrom::Stderr => {
                let mut lock = self.stderr.write().await;

                lock.read_string(Vec::with_capacity(4096),Some(b'\n')).await
                
            },
        }
    }

    pub async fn wait(&self) -> Result<ExitStatus, TrafficStarError>{
        let mut handle = self.handle.write().await;
        Ok(handle.wait().await?)
    }

    pub async fn read(&self, from : AsyncProcessReadFrom, to : &mut [u8]) -> Result<usize,TrafficStarError>{
        match from{
            AsyncProcessReadFrom::Stdout => {
                let mut lock = self.stdout.write().await;
                Ok(lock.read(to).await?)
            },
            AsyncProcessReadFrom::Stderr => {
                let mut lock = self.stderr.write().await;
                Ok(lock.read(to).await?)
            },
        }
    }

    pub async fn write(&self, from : &[u8]) -> Result<usize, TrafficStarError>{
        Ok(self.stdin.write().await.write(from).await?)
    }


    pub async fn write_all(&self, from : &[u8]) -> Result<(), TrafficStarError>{
        Ok(self.stdin.write().await.write_all(from).await?)
    }

    pub async fn kill(&self) -> Result<(), TrafficStarError>{
        let mut handle = self.handle.write().await;
        if let Err(err) = handle.kill().await{
            Err(format!("Unexpected error killing process, error : {}",err).into())
        }else{
            Ok(())
        }
    }
    
    pub async fn send_ctrl_c(&self) -> Result<(), TrafficStarError>{
        if let Some(id) = self.handle.write().await.id(){

            match nix::sys::signal::kill( nix::unistd::Pid::from_raw(id as pid_t),nix::sys::signal::Signal::SIGINT){
                Ok(_) => Ok(()),
                Err(err) => Err(format!("Failed to send signal, error : {}",err).into()),
            }
            

        }else{
            Err(TrafficStarError::msg("Proccess has already exited!".into()))
        }
        
    }
}

impl Drop for ASyncProcess{
    fn drop(&mut self) {
        let handle = self.handle.clone();
        let name = self.name.clone();
        get_singleton_multi().spawn(async move{
            let mut handle = handle.write().await;
            let _ = handle.kill().await;
            sdebug!("Proccess {} dead!",&name);
        });
    }
}



