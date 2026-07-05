use std::sync::Arc;

use futures::{FutureExt, future::Shared};
use tokio::{io::AsyncWriteExt as _, sync::{Notify, futures::OwnedNotified}};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{sdebug};
use trafficstar_logger_macro::StructLoggerName;


use crate::{ get_multi_runtime, trafficstar_stored_join::StoredJoin};

#[derive(StructLoggerName)]
pub struct SinkSender{
    task : Arc<StoredJoin<Result<u128,TrafficStarError>>>,
    stop_notification : Arc<Notify>,
}

impl SinkSender{
    pub fn new(file : tokio::net::TcpStream) -> Result<Self, TrafficStarError>{
        let stop_notifications = Arc::new(Notify::new());
        let task = {
            let stop_notified = stop_notifications.clone().notified_owned();

            std::thread::spawn(move || {get_multi_runtime().unwrap().block_on(async move{
                 Self::run_task(file, stop_notified.shared()).await
            })})
        };
        Ok(Self { task : StoredJoin::new(task).into() , stop_notification: stop_notifications})
    }
    
    pub async fn kill(&self) -> Result<u128, TrafficStarError>{
        self.stop_notification.notify_waiters();
        let task = self.task.clone();
        let res = tokio::task::spawn_blocking(move || {
            task.join()
        }).await;
        res??
    }

    async fn run_task(mut file : tokio::net::TcpStream, stop_notify : Shared<OwnedNotified>) -> Result<u128,TrafficStarError>{
        let buffer = vec![0_u8;8192000];
        let mut sent_bytes : u128 = 0;
        loop{
            let notify = stop_notify.clone();

            tokio::select!{
                v = file.write(&buffer) => {
                    match v{
                        Ok(v) => {
                            //sdebug!("Sent data {}!",v);
                            let _ = file.flush().await;
                            sent_bytes += v as u128;
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
        Ok(sent_bytes)
    }
}

impl Drop for SinkSender{
    fn drop(&mut self) {
        self.stop_notification.notify_waiters();
    }
}