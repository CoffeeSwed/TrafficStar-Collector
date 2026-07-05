use std::sync::{Arc, Condvar, Mutex};

#[derive(Default, Clone)]
pub struct TrafficStarSemaphore{
    count : Arc<Mutex<usize>>,
    condvar : Arc<Condvar>
}

#[derive(Clone)]
pub struct TrafficStarSemaphoreGuard{
    count : Arc<Mutex<usize>>,
    condvar : Arc<Condvar>
}

impl TrafficStarSemaphore{
    pub fn lock(&self, max_reservers : usize) -> TrafficStarSemaphoreGuard{
        let mut count = self.count.lock().unwrap();
        loop {
            if *count >= max_reservers {
                count = self.condvar.wait(count).unwrap();
            } else {
                break;
            }
        }
        *count += 1;

        TrafficStarSemaphoreGuard{
            condvar : self.condvar.clone(),
            count : self.count.clone()
        }
        
    }

    ///Waits for the count to reach that exact value!
    pub fn wait_to(&self, reservers : usize){
        let mut count = self.count.lock().unwrap();
        loop {
            if *count != reservers {
                count = self.condvar.wait(count).unwrap();
            } else {
                break;
            }
        }
    }

    pub fn count(&self) -> usize{
        *self.count.lock().unwrap()
    }
}

impl Drop for TrafficStarSemaphoreGuard{
    fn drop(&mut self) {
        
        let mut count = self.count.lock().unwrap();
        *count -= 1;
        self.condvar.notify_one();
    }
}