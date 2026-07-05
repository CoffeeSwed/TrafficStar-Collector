use std::{ fs::File, io::Read, os::fd::{AsFd, BorrowedFd}, sync::Arc};

use serde::{de::{DeserializeOwned}};
use trafficstar_errors::traffic_star_error::TrafficStarError;


pub struct PipeReceiver{
    stream : Arc<File>,
    buffer : Vec<u8>,

}

impl PipeReceiver{
    
    pub fn new(reader : Arc<File>) -> Result<PipeReceiver, TrafficStarError>{
        
        Ok(PipeReceiver {stream : reader, buffer : vec![]})
    }

   
    

    pub fn read<T : DeserializeOwned>(&mut self) -> Result<T, TrafficStarError> {
        let mut typesize = [0_u8; 8];
        self.stream.read_exact(&mut typesize)?;
        let new_size = usize::from_le_bytes(typesize);
        self.buffer.resize(new_size, 0_u8);
        self.stream.read_exact(&mut self.buffer)?;
        match rmp_serde::decode::from_slice::<T>(&self.buffer){
            Ok(v) => Ok(v),
            Err(err) => Err(TrafficStarError::id_msg("rmp_serde::decode::from_slice".into(),format!("{}",err))),
        }
    }

    pub fn file_id(&self) -> BorrowedFd<'_>{
        self.stream.as_fd()
    }
    
    pub fn take(self) -> Arc<File>{
        self.stream
    }
}