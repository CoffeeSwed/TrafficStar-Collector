use std::{net::ToSocketAddrs, path::PathBuf, sync::{Arc, atomic::AtomicBool}};

use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::tempfiles::TempFileHandler;

use crate::{trafficstar_async_process::ASyncProcess};

#[derive(StructLoggerName)]
///Its always nano
pub struct ASyncTCPDump{
    pub process : Arc<ASyncProcess>,
    pub temp_file : Arc<PathBuf>,
    pub output_file : Arc<PathBuf>,
    saved_file : Arc<AtomicBool>
}

impl ASyncTCPDump{
    
    
    ///Its always nano
    pub async fn new(listen_address : Option<&str>, listen_interface : &str, output_file : PathBuf) -> Result<Self, TrafficStarError>{
        let tempfile = TempFileHandler::get_tempfile("tcpdump").await?;
         let mut command = tokio::process::Command::new("stdbuf");
        command.arg("-oL").arg("tcpdump");
        command.arg("-w").arg(tempfile.as_path().to_str().unwrap());
        command.arg("-s").arg((60+60+18+4+256).to_string()); //60 for IP and TCP header size. 18 for ethernet header size, 4 for margins. 256 for dns name
        command.arg("-i").arg(listen_interface);
        command.arg("--nano");
        if let Some(listen_address) = listen_address{
            command.arg("-l");
            let addrs: Vec<_> = listen_address
            .to_socket_addrs()
            .map_err(|_| TrafficStarError::msg("Invalid ipv4 address counstructed!".into()))?
            .collect();
            let address: &mut &_ = &mut addrs.first().unwrap();
            command
                .arg("(dst")
                .arg(address.ip().to_string())
                .arg("and")
                .arg("dst port")
                .arg(address.port().to_string() + ")");
            command.arg("or");
            command
                .arg("(src")
                .arg(address.ip().to_string())
                .arg("and")
                .arg("src port")
                .arg(address.port().to_string() + ")");
        }
        let process = ASyncProcess::new(command, "TCPDUMP".into()).await?;
        Ok(Self{
            process,
            output_file : Arc::new(output_file),
            temp_file : Arc::new(tempfile),
            saved_file : Arc::new(AtomicBool::new(false))
        })

    }


    pub async fn save_and_exit(&self) -> Result<(), TrafficStarError>{
        self.process.send_ctrl_c().await?;
        if !self.process.wait().await?.success(){
            return Err(TrafficStarError::msg("TCPDump exited with errors!".into()))
        }
        let mut original_file = File::open(self.temp_file.as_path()).await?;
        let mut destination = File::create(self.output_file.as_path()).await?;
        let mut buffer : Vec<u8> = vec![0_u8;4096];
        while let size = original_file.read(&mut buffer).await? && size > 0{
            destination.write_all(&buffer[..size]).await?;
        }
        self.saved_file.store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}

impl Drop for ASyncTCPDump{
    fn drop(&mut self) {
        if !self.saved_file.load(std::sync::atomic::Ordering::Acquire){
            let file = self.output_file.clone();
            if let Err(err) = std::fs::remove_file(file.as_path()){
                serror!("Failed deleting old file {:?}, error : {}!",file.as_path(),err);
            }
            else{
                sinfo!("Deleted bad tcpdump file : {:?}",self.output_file.as_path());
            }
        }
    }
}