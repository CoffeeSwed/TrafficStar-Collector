/*

*/

use std::{path::{Path, PathBuf}, process::ExitStatus, str::FromStr, sync::Arc, time::Duration};

use nix::{errno::Errno, sys::signal::{Signal, killpg}, unistd::Pid};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::process::Command;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_interface::{LinkType, reservation::{PeerForwardingReservation, ReservationController}};
use trafficstar_logger::{serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{async_run_command, get_singleton_multi, run_command, tempfiles::directory_copies::{CopyHandlerHolder, DirectoryCopyHolder}, trafficstar_networking::interface::TrafficStarInterfaceName};

use crate::trafficstar_async_process::ASyncProcess;
use serde_with::{serde_as};
use serde_with::DurationSeconds;

#[serde_as]
#[derive(Deserialize,Serialize,Default, Clone, PartialEq)]
pub struct MullvadBrowserSettings{
    pub test_script : PathBuf,
    pub websites : Vec<String>,
    pub rate : Option<String>,
    #[serde_as(as = "DurationSeconds")]
    pub max_time : Duration
}

impl std::fmt::Display for MullvadBrowserSettings{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"MullvadBrowserSettings{{test_script = {{{}}} ,websites = {{{:?}}}, max_time = {{{:?}}}}}",
            self.test_script.as_path().to_str().unwrap_or("?"),
            self.websites.clone(),
            self.max_time
        )
    }
}

impl MullvadBrowserSettings{
    pub fn generate_runs(&self) -> Vec<MullvadBrowserRun>{
        let mut res = Vec::new();
        for website in &self.websites{
            res.push(
                MullvadBrowserRun { test_script: self.test_script.clone(),
                 website: website.clone(), 
                 max_progress_time: self.max_time 
                }
            );
        }
        res
    }
}

#[serde_as]
#[derive(Deserialize,Serialize,Default, Clone, PartialEq)]
pub struct MullvadBrowserRun{
    pub test_script : PathBuf,
    pub website : String,
    #[serde_as(as = "DurationSeconds")]
    pub max_progress_time : Duration
}

impl std::fmt::Display for MullvadBrowserRun{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"MullvadBrowserSettings{{test_script = {{{}}} ,website = {{{:?}}}, max_time = {{{:?}}}}}",
            self.test_script.as_path().to_str().unwrap_or("?"),
            self.website,
            self.max_progress_time
        )
    }
}

#[derive(StructLoggerName)]
pub struct MullvadBrowserProccess{
    pub process : Arc<ASyncProcess>,
    pub proccess_id : Pid,
    pub vpair : Arc<PeerForwardingReservation>,
    pub browser_copy : Arc<CopyHandlerHolder>,
    pub profile_dir : Arc<TempDir>,
    pub executable : Arc<String>,
    pub timeout : Duration
}

impl MullvadBrowserProccess{
    
   

    pub async fn new(
        script_path : &Path, 
        interface_name : TrafficStarInterfaceName, 
        is_layer_3 : Option<LinkType>, 
        timeout : Duration
        )
        //file_name : String) 
        -> Result<Self, TrafficStarError>{
        let mut command: Command = Command::new("python3");
        command.arg(script_path);

        
         let source_path = std::env::var("MULLVAD").map_err(
            |e| TrafficStarError::from(format!("Missing mullvad browser environment variable MULLVAD? Err : {}",e)))?;
        let source_path = PathBuf::from_str(&source_path).map_err(|_| TrafficStarError::msg("Bad browser directory given!".into()))?;



        let browser_temp = DirectoryCopyHolder::get_handler(&source_path).await.get_copy(Some(512)).await?;
        let profile_temp = Arc::new(TempDir::new("browser-profile")?);
        command.process_group(0);
            unsafe { 
                command.pre_exec(move || {
                    
                    nix::sched::unshare(nix::sched::CloneFlags::CLONE_NEWNET
                            )
                    .map_err(|e| TrafficStarError::from(format!("{}",e)))?;

                    /* nix::unistd::setsid()
                        .map_err(|e| TrafficStarError::from(format!("{}", e)))?;
                     */

     
                Ok(())
            }) 
        };
        
        let peer_forwarding_reservation = tokio::task::spawn_blocking(
            move || {
            match ReservationController::instance().
                create_peer_forwarding_reservation(interface_name, is_layer_3){
                    Ok(v) => Ok(v),
                    Err(err) => Err(err),
                }
            }
        ).await.unwrap()?;
        //let executable_loc = format!("{}/firefox/firefox",browser_temp.get_path().to_str().unwrap());
        let executable_loc = format!("{}/Browser/mullvadbrowser.real",browser_temp.get_path().to_str().unwrap());

        command.env("DISPLAY_NUMBER", format!(":{}",peer_forwarding_reservation.table_id.get_table()));
        command.env("DISPLAY", format!(":{}",peer_forwarding_reservation.table_id.get_table()));
        command.env("BROWSER", &executable_loc);
        command.env("PROFILE", profile_temp.path().to_str().unwrap());
        command.env("PYVIRTUALDISPLAY_DISPLAYFD", "1");
              

        let child = ASyncProcess::new(command, "MullvadBrowser".into()).await?;
        
        let child_lock_id = match child.handle.write().await.id(){
            Some(v) => v,
            None => return Err(TrafficStarError::msg("No pid was found, the process has exited!".into())),
        };


        let proccess_id = Pid::from_raw(child_lock_id as i32);
        
        let child_name = peer_forwarding_reservation.vchild_name.get_name();
        let child_ip = peer_forwarding_reservation.vchild_ip.get_ip();
                
        async_run_command("ip", vec!["link","set",child_name.as_string(),"netns",&proccess_id.to_string()]).await?;
        async_run_command("nsenter", vec!["-t",&proccess_id.to_string(),"-n",
        "ip","addr","add",&(child_ip.to_string()+"/8"),"dev",child_name.as_string()]).await?;
        async_run_command("nsenter", vec!["-t",&proccess_id.to_string(),"-n",
        "ip","link","set","up",child_name.as_string()]).await?;
        async_run_command("nsenter", vec!["-t",&proccess_id.to_string(),"-n",
        "ip","link","set","up","lo"]).await?;
        async_run_command("nsenter", vec!["-t",&proccess_id.to_string(),"-n",
        "ip","route","add","default","via",&peer_forwarding_reservation.vparent_ip.get_ip().to_string()]).await?;
        //run_command("nsenter", vec!["-t",&proccess_id.to_string(),"-n",
        //"ip","link","set","dev",child_name.as_string(),"arp","off"])?;
        //async_run_command("nsenter", vec!["-t", &proccess_id.to_string(), "-n", "sleep", "infinity"]).await?;
        Ok(
            Self{
                process : child,
                proccess_id,
                vpair : peer_forwarding_reservation,
                browser_copy: Arc::new(browser_temp),
                profile_dir : profile_temp,
                executable : Arc::new(executable_loc),
                timeout
            }
        )
    }

    #[allow(clippy::byte_char_slices)]
    pub async fn write(&mut self, string : &str) -> Result<(),TrafficStarError>{
        match tokio::time::timeout(self.timeout, async {
        self.process.write_all(string.as_bytes()).await?;
        self.process.write_all(&[b'\n']).await
        }).await{
            Ok(v) => v,
            Err(_) => Err(format!("Timeout {:?} reached!",self.timeout).into()),
        }
    }

    pub async fn read_line(&mut self) -> Result<String,TrafficStarError>{
        match tokio::time::timeout(self.timeout, async {
            self.process.read_line(crate::trafficstar_async_process::AsyncProcessReadFrom::Stdout).await
        }).await{
            Ok(v) => Ok(v?.trim_ascii_end().to_string()),
            Err(_) => Err(format!("Timeout {:?} reached!",self.timeout).into()),
        }
    }
    
    pub async fn kill(&mut self) -> Result<(),TrafficStarError>{
        let res = self.process.kill().await;
        let _ = run_command("pkill", vec!["-f",&self.executable]);
        res
    }

    pub async fn wait(&mut self) -> Result<ExitStatus,TrafficStarError>{
        self.process.wait().await
    }
}
impl Drop for MullvadBrowserProccess{
    fn drop(&mut self) {
       let executable = self.executable.clone();
       let directory = self.browser_copy.clone();
       let proccess = self.process.clone();
       let proccess_id = self.proccess_id;
       let reservation = self.vpair.clone();
        get_singleton_multi().spawn(async move{
            let _ = async_run_command("pkill", vec!["-f",&executable]).await;
            if let Err(err) = killpg(proccess_id, Signal::SIGKILL){
                if let Some(errno) = err.as_errno() && errno == Errno::ESRCH{
                    sinfo!("Proccess group already dead!");
                    return
                }
                serror!("Failure killing proccess group, error : {}",err);
            }else{
                sinfo!("Killed proccess group!");
            }
            let _ = proccess.kill().await;

            drop(directory);
            drop(reservation);

        });

       
    }
} 
