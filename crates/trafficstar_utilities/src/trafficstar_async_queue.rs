use std::{collections::VecDeque, sync::Arc};

use tokio::sync::{Mutex, Semaphore};
use trafficstar_errors::traffic_star_error::TrafficStarError;


#[derive(Clone)]
pub struct AsyncSharedQueue<T : Send + Sync>{
    in_queue : Arc<Mutex<VecDeque<T>>>,
    semaphore : Arc<Semaphore>,
}

impl<T: Send + Sync> AsyncSharedQueue<T>{
    pub fn new() -> Self{
        Self { 
            in_queue: Arc::new(Mutex::new(VecDeque::new())), 
            semaphore : Arc::new(Semaphore::new(0))
        }
    }
    
    pub fn push(&self,value : T){
        loop{
            if let Ok(mut lock) = self.in_queue.try_lock(){
                lock.push_back(value);
                self.semaphore.add_permits(1);
                drop(lock);
                break;
            }
        }
    }

    pub async fn pop(&self) -> Result<T,TrafficStarError>{
        if let Ok(lock) = self.semaphore.acquire().await{
            lock.forget();
            loop{
                if let Ok(mut queue_lock) = self.in_queue.try_lock(){
                    loop{
                        let p = queue_lock.pop_front();
                        if let Some(value) = p{
                            return Ok(value);
                        }
                    }
                }
            }
        }else{
            if let mut queue_lock = self.in_queue.lock().await && let Some(item) = queue_lock.pop_front(){
                Ok(item)
            }else{
                Err("Queue has been closed!".into())
            }
        }
    }

    pub fn close(&self){
        self.semaphore.close();
    }
}

impl<T: Send + Sync> Default for AsyncSharedQueue<T>{
    fn default() -> Self {
        Self::new()
    }
}