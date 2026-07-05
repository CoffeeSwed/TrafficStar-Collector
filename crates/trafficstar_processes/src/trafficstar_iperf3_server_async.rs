use std::sync::Arc;

use tokio::process::Command;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_utilities::get_singleton_multi;

use crate::trafficstar_async_process::ASyncProcess;

pub struct Iperf3Server{
    pub proccess : Arc<ASyncProcess>
}

impl Iperf3Server{
    pub async fn new(interface : &str, port : u16) -> Result<Self,TrafficStarError>{
        let mut command : Command = Command::new("iperf3");
        command.arg("-s").arg("-p").arg(port.to_string()).arg("--bind-dev").arg(interface);
        Ok(Self{
            proccess : ASyncProcess::new(command, "Iperf3Server".into()).await?
        })
    }
}

impl Drop for Iperf3Server{
    fn drop(&mut self) {
        let proccess = self.proccess.clone();
        get_singleton_multi().spawn(async move{
            let _ = proccess.send_ctrl_c().await;
            let _ = proccess.wait().await;
        });
    }
}