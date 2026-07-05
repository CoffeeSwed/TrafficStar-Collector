/*
xhost +SI:localuser:trafficstar

docker run --rm -it --cap-add=NET_ADMIN --device /dev/net/tun trafficstar-test
*/

mod test;
pub mod tor_vpn_config;
use std::{env, mem::MaybeUninit, process::Stdio, str::FromStr, sync::{Arc, Once}, time::Duration};

use tempdir::TempDir;
use tokio::{process::Command, sync::Mutex, task::JoinHandle};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{panicerror, sdebug, serror};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_processes::trafficstar_async_process::ASyncProcess;
use trafficstar_utilities::{async_run_command, create_single_runtime, get_singleton_multi, trafficstar_networking::{create_tunnel, drop_tunnel, interface::TrafficStarInterfaceName}};

use crate::TorInterfaceConfig;
pub const ONIONMASQ_ENV :&str = "ONIONMASQ";
//pub mod torhandle;



#[allow(unused)]
#[derive(StructLoggerName)]
pub struct TorProcess{
  pub reservation : Arc<TorInterfaceConfig>,
  pub handle : Arc<ASyncProcess>,
  pub state_dir : TempDir,
  pub cache_dir : TempDir,
  pub debugger : Option<JoinHandle<()>>

}


impl TorProcess{
    #[allow(static_mut_refs)]
    pub async fn start_manager(){
        static mut SINGLETON: MaybeUninit<(Arc<std::process::Child>,Arc<TorInterfaceConfig>)> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        let _ = tokio::task::spawn_blocking(|| {
        // SAFETY:
        // Needed to create a singleton of an initializable as a static.
        unsafe {
            ONCE.call_once(|| {
                let reservation = create_single_runtime().unwrap().block_on(TorInterfaceConfig::new(TrafficStarInterfaceName::from_str("none").unwrap()));
                
                 let mut command = std::process::Command::new(match env::var(ONIONMASQ_ENV){
                        Ok(v) => v,
                        Err(_) => panicerror!("Missing environment variable {} for TorDevice",ONIONMASQ_ENV),
                    });
                    command.stdout(Stdio::null());
                    command.stderr(Stdio::null());
                    command.stdin(Stdio::null());
                    //command.arg("--bind-device").arg(reservation.out_interface.as_string())
                    //command.arg("--state-directory").arg(statedir.path().to_path_buf().to_str().unwrap())
                    //command.arg("--cache-directory").arg(cachedir.path().to_path_buf().to_str().unwrap())
                    command.arg("--fwmark").arg(reservation.fwmark_resv.get_mark().to_string());
                    //.arg("--device").arg(reservation.interface_name.get_name().as_string());
                let proccess = command.spawn().unwrap();
                std::thread::sleep(Duration::from_secs(5));
                SINGLETON.write((Arc::new(proccess),Arc::new(reservation)));

            });

            SINGLETON.assume_init_mut();
        }
        }).await;
    }

     #[allow(static_mut_refs)]
    pub async fn get_lock() -> Arc<Mutex<u32>>{
        static mut SINGLETON: MaybeUninit<Arc<Mutex<u32>>> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        tokio::task::spawn_blocking(|| {
        // SAFETY:
        // Needed to create a singleton of an initializable as a static.
        unsafe {
            ONCE.call_once(|| {
               
                SINGLETON.write(Arc::new(Mutex::new(0)));

            });

            SINGLETON.assume_init_ref().clone()
        }
        }).await.unwrap()
    }
    
    pub async fn new(reservation : TorInterfaceConfig) -> Result<Arc<Self>, TrafficStarError>{
        Self::start_manager().await;
        let reservation_clone = reservation.clone();
        tokio::task::spawn_blocking(move || {
            create_tunnel(&reservation_clone.interface_name.get_name(), Some(reservation_clone.ipv4_addr.get_ip()), None)
        }).await.unwrap()?;
        let statedir = match TempDir::new("onionmasq-state")
            {
                Ok(v) => v,
                Err(err) => return Err(TrafficStarError::msg(format!("Failed to create tempdir for state, error : {}",err))),
            }
        ;
        let cachedir = match TempDir::new("onionmasq-cache")
            {
                Ok(v) => v,
                Err(err) => return Err(TrafficStarError::msg(format!("Failed to create tempdir for state, error : {}",err))),
            }
        ;
        
        
        let mut command = Command::new(match env::var(ONIONMASQ_ENV){
            Ok(v) => v,
            Err(_) => return Err(TrafficStarError::msg(format!("Missing environment variable {}",ONIONMASQ_ENV))),
        });
        let statedir_path = statedir.path().to_path_buf();
        command.arg("--bind-device").arg(reservation.out_interface.as_string())
        .arg("--state-directory").arg(statedir.path().to_path_buf().to_str().unwrap())
        //.arg("--cache-directory").arg(cachedir.path().to_path_buf().to_str().unwrap())
        .arg("--fwmark").arg(reservation.fwmark_resv.get_mark().to_string())
        .arg("--device").arg(reservation.interface_name.get_name().as_string());
        let lock = Self::get_lock().await;
        let guard = lock.lock().await;
        let proccess = ASyncProcess::new(command, "OnionMasq".into()).await?;
        let res = Arc::new(Self{
            reservation : Arc::new(reservation),
            handle : proccess.clone(),
            state_dir : statedir,
            cache_dir : cachedir,
            debugger : Some(tokio::task::spawn(Self::read_outs(proccess)))
        });


        match tokio::time::timeout(Duration::from_secs(60*10),async_run_command("curl", vec!["https://google.com","--interface",res.reservation.interface_name.get_name().as_string()])).await{
            Ok(v) => match v{
                Ok(_) => {},
                Err(_) => {
                    res.stop().await?;
                    drop(guard);
                    return Err("Bad exit status!".into())
                },
            },
            Err(_) => {
                res.stop().await?;
                drop(guard);
                return Err(TrafficStarError::msg("Timeout of creation!".to_string()))},
        };
        drop(guard);
        sdebug!("Using statedirectory : {:?}",statedir_path);
        Ok(res)
    }

    async fn read_outs(proccess : Arc<ASyncProcess>){
        loop{
            let stdout = proccess.read_line(trafficstar_processes::trafficstar_async_process::AsyncProcessReadFrom::Stdout);
            let stderr = proccess.read_line(trafficstar_processes::trafficstar_async_process::AsyncProcessReadFrom::Stderr);
            tokio::select! {
                 msg = stdout => {
                    match msg{
                        Ok(v) => {
                            sdebug!("{}",v.trim_ascii_end()); 
                        }
                        Err(_) => {
                            break 
                        }
                    }
                },

                 msg = stderr => {
                    match msg{
                        Ok(v) => {
                            serror!("{}",v.trim_ascii_end());
                        }
                        Err(_) => {
                            break 
                        }
                    }
                },
            }
        }
    }
    

   

    ///Interface name
    pub fn name(&self) -> TrafficStarInterfaceName{
        self.reservation.interface_name.get_name()
    }

    pub async fn stop(&self) -> Result<(), TrafficStarError>{
        self.handle.send_ctrl_c().await?;
        self.handle.wait().await?;
        Ok(())
    }
}

impl Drop for TorProcess{
    fn drop(&mut self) {
        let interface = self.reservation.clone();
        let handle = self.handle.clone();
        if let Some(debugger) = self.debugger.take(){
            debugger.abort();
        }
        get_singleton_multi().spawn(async move {
            let interface_name = interface.interface_name.get_name();
            let _ = handle.send_ctrl_c().await;
            let _ = handle.wait().await;
            let _ = get_singleton_multi().spawn_blocking(move || {drop_tunnel(&interface_name)}).await;
            
            drop(interface);
        });
    }
}