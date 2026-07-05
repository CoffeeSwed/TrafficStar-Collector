use std::{sync::{Arc, Mutex}, thread::JoinHandle};

use trafficstar_errors::{traffic_star_error::TrafficStarError, trafficstar_error_types::TrafficStarErrorTypes};
use trafficstar_logger::panicerror;



pub struct StoredJoin<T> where T :  Send{
    value : Arc<Mutex<Option<Result<T,TrafficStarError>>>>,
    handle : Mutex<Option<JoinHandle<T>>>,
}

impl<T> StoredJoin<T>
where T : Send
{
    pub fn new(handle : JoinHandle<T>) -> StoredJoin<T>{
       
        StoredJoin { value: Arc::new(Mutex::new(None)),handle : Mutex::new(Some(handle))}
    }

    pub fn join(&self) -> Result<T, TrafficStarError> where T : Clone{
        loop {
            if let Ok(mut value) = self.value.lock(){
                if let Some(value_v) = value.clone(){
                    drop(value);
                    return value_v;
                }else {
                    if let Ok(mut handle_guard) = self.handle.lock(){
                        if let Some(handle) = handle_guard.take(){
                            *value = Some(match handle.join(){
                                Ok(v) => {Ok(v)},
                                Err(err) => {
                                    let err = 
                                    Arc::new(
                                        TrafficStarErrorTypes::JoinHandleBad { error: format!("{:?}",err)});
                                    Err(TrafficStarError::enums(err))
                                },
                            });
                        }
                    }else{
                        panicerror!("Poisioned mutex!");
                    }
                }
                drop(value);
            }else{
                panicerror!("Poisioned mutex!");
            }
        }
    }

}


