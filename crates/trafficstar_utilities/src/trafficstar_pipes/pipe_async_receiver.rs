

use futures::AsyncReadExt;
use serde::{de::{DeserializeOwned}};
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::trafficstar_files::file_handler::FileHandler;


pub struct AsyncPipeReceiver{
    stream : FileHandler,
    buffer : Vec<u8>,

}

impl AsyncPipeReceiver{
    
    pub fn new(reader : FileHandler) -> Result<AsyncPipeReceiver, TrafficStarError>{
       
        Ok(AsyncPipeReceiver {stream : reader, buffer : vec![0_u8;0]})
    }


    pub async fn read<T : DeserializeOwned>(&mut self) -> Result<T, TrafficStarError> {
        let mut typesize = [0_u8; 8];
        self.stream.read_exact(&mut typesize).await?;
        let new_size = usize::from_le_bytes(typesize);
        self.buffer.resize(new_size, 0_u8);
        self.stream.read_exact(&mut self.buffer).await?;
        match rmp_serde::decode::from_slice::<T>(&self.buffer){
            Ok(v) => Ok(v),
            Err(err) => Err(TrafficStarError::id_msg("rmp_serde::decode::from_slice".into(),format!("{}",err))),
        }
        
    }

    
    pub fn take(self) -> FileHandler{
        self.stream
    }

    ///Creates a new version with its own ReadBuffer
    pub fn cool(&self){
        
    }
}