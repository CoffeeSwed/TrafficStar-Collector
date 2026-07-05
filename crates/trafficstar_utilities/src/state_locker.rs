use std::sync::Arc;

use tokio::sync::{Mutex, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

struct StateLockerInner<T>{
    read_lock : Option<OwnedRwLockReadGuard<T>>,
    write_lock : Option<OwnedRwLockWriteGuard<T>>
}

impl<T> Drop for StateLockerInner<T>{
    fn drop(&mut self) {
        self.read_lock.take();
        self.write_lock.take();
    }
}

///Has a shared state
#[derive(Clone)]
pub struct StateLocker<T>{
    rwlock : Arc<RwLock<T>>,
    inner : Arc<Mutex<StateLockerInner<T>>>
}

impl<T> StateLocker<T>{
    pub fn new(data : T) -> Self{
        Self { 
            rwlock: Arc::new(RwLock::new(data)),
            inner : Arc::new(Mutex::new(StateLockerInner{
                read_lock : None,
                write_lock : None
            }))
        }
    }

    pub async fn view(&self){
        let mut inner = self.inner.lock().await;
        if inner.read_lock.is_some(){
            return;
        }
        if let Some(write_lock) = inner.write_lock.take(){
            drop(write_lock);
        }
        inner.read_lock = Some(self.rwlock.clone().read_owned().await);
        
    }

    pub async fn block(&self){
        let mut inner = self.inner.lock().await;
        if inner.write_lock.is_some(){
            return;
        }
        if let Some(read_lock) = inner.read_lock.take(){
            drop(read_lock);
        }
        inner.write_lock = Some(self.rwlock.clone().write_owned().await)
    }

    pub async fn uninterested(&self){
        let mut inner = self.inner.lock().await;
        inner.read_lock.take();
        inner.write_lock.take();
    }

    ///Doesn't share the lock/hold but the exclusion. 
    pub fn new_shared(&self) -> StateLocker<T>{
        Self { 
            rwlock: self.rwlock.clone(),
            inner : Arc::new(Mutex::new(StateLockerInner{
                read_lock : None,
                write_lock : None
            }))
        }
    }
}