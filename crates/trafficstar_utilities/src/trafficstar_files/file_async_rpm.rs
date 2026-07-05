use std::sync::Arc;

use futures::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{sync::Mutex};
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::trafficstar_files::file_handler::FileHandler;


pub struct FileAsyncRpm{
    pub file : Arc<Mutex<FileHandler>>,
    pub buffer : Arc<Mutex<Vec<u8>>>,
}

const BYTESBEFORECHECK : usize = 8192_usize;

impl FileAsyncRpm{
    pub fn new(file : FileHandler) -> Self{
        Self{
            file : Arc::new(Mutex::new(file)),
            buffer : Arc::new(Mutex::new(Vec::with_capacity(1024)))
        }
    }

    pub async fn read<T : DeserializeOwned>(&self) -> Result<T, TrafficStarError> {
        let mut typesize = [0_u8; size_of::<usize>()];
        let mut file = self.file.lock().await;
        let mut buffer = self.buffer.lock().await;
        file.read_exact(&mut typesize).await?;
        let new_size = usize::from_le_bytes(typesize);
        if new_size > BYTESBEFORECHECK{
            file.read_exact(&mut typesize).await?;
            let confirmed_size = (-isize::from_le_bytes(typesize)) as usize;

            if new_size != confirmed_size || new_size > isize::MAX as usize{
                return Err(TrafficStarError::msg("Received corrupt usize, is the stream corrupted?".into()))
            }
        }
        
        
        buffer.resize(new_size, 0_u8);
        file.read_exact(&mut buffer).await?;
        match rmp_serde::decode::from_slice::<T>(&buffer){
            Ok(v) => Ok(v),
            Err(err) => Err(TrafficStarError::id_msg("rmp_serde::decode::from_slice".into(),format!("{}",err))),
        }
    
    }

    pub async fn send<T : Serialize>(&self, data : T) -> Result<(), TrafficStarError> {
        let data = match rmp_serde::encode::to_vec(&data){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::id_msg("rmp_serde::encode::to_vec".into(),format!("{}",err))),
        };
        let mut file = self.file.lock().await;
        file.write_all(&data.len().to_le_bytes()).await?;
        if data.len() > BYTESBEFORECHECK{
            let len = -(data.len() as isize);
            file.write_all(&len.to_le_bytes()).await?;
        }
        file.write_all(&data).await?;
        file.flush().await?;
        Ok(())
    }
}