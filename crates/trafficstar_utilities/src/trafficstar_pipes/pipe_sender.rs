use std::{ fs::File, io::{Write}, os::fd::{AsFd, BorrowedFd}, sync::Arc};

use serde::{Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;


pub struct PipeSender{
    stream : Arc<File>,
}

impl PipeSender{
    
    pub fn new(writer : Arc<File>) -> Result<Self,TrafficStarError>{
        Ok(Self { stream : writer})
    }

   

    pub fn send<T : Serialize>(&mut self, data : T) -> Result<(), TrafficStarError> {
        let data = match rmp_serde::encode::to_vec(&data){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::id_msg("rmp_serde::encode::to_vec".into(),format!("{}",err))),
        };
        self.stream.write_all(&data.len().to_le_bytes())?;
        self.stream.write_all(&data)?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn file_id(&self) -> BorrowedFd<'_>{
        self.stream.as_fd()
    }

    pub fn take(self) -> Arc<File>{
        self.stream
    }
}