use std::sync::Arc;

use async_condvar_fair::{Condvar};
use tokio::sync::Mutex;

use crate::get_singleton_multi;


#[derive(Default, Clone)]
pub struct TrafficStarAsyncSemaphore{
    count : Arc<Mutex<usize>>,
    condvar : Arc<Condvar>
}

#[derive(Clone)]
pub struct TrafficStarAsyncSemaphoreGuard{
    count : Arc<Mutex<usize>>,
    condvar : Arc<Condvar>
}

impl TrafficStarAsyncSemaphore{
    pub async fn lock(&self, max_reservers : usize) -> TrafficStarAsyncSemaphoreGuard{
        let mut guard = self.count.clone().lock_owned().await;
        loop {
            if *guard >= max_reservers {
                guard = self.condvar.wait((guard,self.count.clone())).await;
            } else {
                break;
            }
        }
        *guard += 1;

        TrafficStarAsyncSemaphoreGuard{
            condvar : self.condvar.clone(),
            count : self.count.clone()
        }
        
    }

    ///Waits for the count to reach that exact value!
    pub async fn wait_to(&self, reservers : usize){
        let mut count = self.count.clone().lock_owned().await;
        loop {
            if *count != reservers {
                count = self.condvar.wait((count,self.count.clone())).await;
            } else {
                break;
            }
        }
    }

    pub async fn count(&self) -> usize{
        *self.count.lock().await
    }
}

impl Drop for TrafficStarAsyncSemaphoreGuard{
    fn drop(&mut self) {
        let count = self.count.clone();
        let condvar = self.condvar.clone();
        get_singleton_multi().spawn(async move{
            let mut count = count.lock().await;
            *count -= 1;
            condvar.notify_one();
        });
        
    }
}