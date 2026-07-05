use std::{sync::{Arc, atomic::AtomicUsize}, time::{Duration}};

use futures::{FutureExt, future::Shared};
use tokio::{io::AsyncReadExt, sync::{Notify, futures::OwnedNotified}, time::Instant};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{sdebug, serror};
use trafficstar_logger_macro::StructLoggerName;

use crate::{get_multi_runtime, trafficstar_stored_join::StoredJoin};

#[derive(StructLoggerName)]
pub struct SinkReceiver{
    task : Arc<StoredJoin<()>>,
    stop_notification : Arc<Notify>,
    speed : Arc<AtomicUsize>,
    total_bytes : Arc<AtomicUsize>
}

impl SinkReceiver{
    pub fn new(file : tokio::net::TcpStream) -> Result<Self, TrafficStarError>{
        let stop_notifications = Arc::new(Notify::new());
        let stop_notify = stop_notifications.clone().notified_owned();
        let speed = Arc::new(AtomicUsize::new(0));
        let total_bytes = Arc::new(AtomicUsize::new(0));
        let task = {
            let speed = speed.clone();
            let total_bytes = total_bytes.clone();
            std::thread::spawn(move || {
                get_multi_runtime().unwrap().block_on(async move{
                if let Err(err) = Self::run_task(file, stop_notify.shared(),speed, total_bytes).await{
                    serror!("SinkReceiver Error : {}",err);
                }
            });
        })
        };
        Ok(Self { task : StoredJoin::new(task).into() , stop_notification: stop_notifications,speed,total_bytes })
    }

    pub fn new_listener(file : tokio::net::TcpListener) -> Result<Self, TrafficStarError>{
        let stop_notifications = Arc::new(Notify::new());
        let stop_notify = stop_notifications.clone().notified_owned();
        let speed = Arc::new(AtomicUsize::new(0));
        let total_bytes = Arc::new(AtomicUsize::new(0));
        let task = {
            let speed = speed.clone();
            let total_bytes = total_bytes.clone();
            std::thread::spawn(move || {
                get_multi_runtime().unwrap().block_on(async move{
                let stop_notify = stop_notify.shared();

                let stream : tokio::net::TcpStream = {
                    let stopped = stop_notify.clone();
                    tokio::select! {
                        _ = stopped => {
                            sdebug!("Stopped!");
                            return;
                        }
                        v = file.accept() => {
                            match v{
                                Ok(v) => v.0,
                                Err(err) => {
                                    serror!("Received error accepting the client : {}",err);
                                    return;
                                },
                            }
                        } 
                    }
                };
                if let Err(err) = Self::run_task(stream, stop_notify,speed, total_bytes).await{
                    serror!("SinkReceiver Error : {}",err);
                }
            });
        })
        };
        Ok(Self { task : StoredJoin::new(task).into() , stop_notification: stop_notifications,speed, total_bytes})
    }

    pub fn get_speed(&self) -> usize{
        self.speed.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn get_speed_mbits(&self) -> f64{
        (self.get_speed() as f64*8.0) / (1000000.0)
    }

    pub fn get_transfered_mebibytes(&self) -> f64{
        let total_bytes = self.total_bytes.load(std::sync::atomic::Ordering::Acquire) as f64;
        total_bytes / (1024.0*1024.0)
    }
    
    pub async fn kill(&self){
        self.stop_notification.notify_waiters();
        let task = self.task.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = task.join();
        }).await;
    }

    async fn run_task(mut file : tokio::net::TcpStream, stop_notify : Shared<OwnedNotified>, speed : Arc<AtomicUsize>, total_bytes : Arc<AtomicUsize>) -> Result<(),TrafficStarError>{
        let mut buffer = vec![0_u8;8192000];
        sdebug!("Listening!");
        let mut bytes_read = 0;
        let mut timer = Instant::now().checked_add(Duration::from_secs(1)).unwrap();
        
        loop{
            let notify = stop_notify.clone();
            let speed = speed.clone();
            let bytes = total_bytes.clone();
            tokio::select!{
                v = file.read(&mut buffer) => {
                    match v{
                        Ok(v) => {
                            //sdebug!("Read {}",v);
                            if v == 0{
                                break;
                            }
                            bytes_read += v;
                            bytes.fetch_add(v, std::sync::atomic::Ordering::AcqRel);

                            if Instant::now() > timer{
                                let time =  timer.elapsed().as_millis() + 1000;
                                
                                speed.update(std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire, |_| bytes_read*1000/(time as usize));
                                timer = Instant::now().checked_add(Duration::from_secs(1)).unwrap();
                                bytes_read = 0;
                            }
                        },
                        Err(_) => {
                            break;
                        },
                    }
                },
                _ = notify => {
                    sdebug!("Received kill!");
                    break;
                }
            };

        }
        Ok(())
    }
}

impl Drop for SinkReceiver{
    fn drop(&mut self) {
        self.stop_notification.notify_waiters();
    }
}