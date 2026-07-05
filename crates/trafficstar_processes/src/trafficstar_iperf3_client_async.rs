use std::{process::ExitStatus, sync::Arc};

use tokio::process::Command;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_utilities::get_singleton_multi;

use crate::trafficstar_async_process::ASyncProcess;

pub struct Iperf3Client{
    pub proccess : Arc<ASyncProcess>
}

impl Iperf3Client{
    pub async fn new(interface : &str,dest : &str, time : usize) -> Result<Self,TrafficStarError>{
        let mut command : Command = Command::new("iperf3");
        let mut destination = dest.split(":");
        let ipv4 = match destination.next(){
            Some(v) => v,
            None => return Err("Destination is empty!".into()),
        };
        let port = match destination.next(){
            Some(v) => v,
            None => return Err(format!("Missing port in destination {}!",dest).into()),
        };
        command.arg("-c").arg(ipv4).arg("-p").arg(port).arg("--bind-dev").arg(interface).arg("-t").arg(time.to_string());
        Ok(Self{
            proccess : ASyncProcess::new(command, "Iperf3Client".into()).await?
        })
    }

    pub async fn wait(&self) -> Result<ExitStatus,TrafficStarError>{
        self.proccess.wait().await
    }
}

impl Drop for Iperf3Client{
    fn drop(&mut self) {
        let proccess = self.proccess.clone();
        get_singleton_multi().spawn(async move{
            let _ = proccess.send_ctrl_c().await;
            let _ = proccess.wait().await;
        });
    }
}