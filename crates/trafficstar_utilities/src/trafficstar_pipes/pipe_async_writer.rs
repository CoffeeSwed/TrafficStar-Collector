

use futures::AsyncWriteExt;
use serde::{Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::trafficstar_files::file_handler::FileHandler;

pub struct AsyncPipeSender{
    stream : FileHandler,
}

impl AsyncPipeSender{
    
    pub fn new(reader : FileHandler) -> Result<AsyncPipeSender, TrafficStarError>{
       
        Ok(AsyncPipeSender {stream : reader})
    }
    

    

    
    
    pub async fn send<T : Serialize>(&mut self, data : T) -> Result<(), TrafficStarError> {
          let data = match rmp_serde::encode::to_vec(&data){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::id_msg("rmp_serde::encode::to_vec".into(),format!("{}",err))),
        };
        self.stream.write_all(&data.len().to_le_bytes()).await?;
        self.stream.write_all(&data).await?;
        Ok(())
        
    }

    
    pub fn take(self) -> FileHandler{
        self.stream
    }
}