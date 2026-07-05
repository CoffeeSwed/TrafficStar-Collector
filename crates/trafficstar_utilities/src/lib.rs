/*


*/

pub mod trafficstar_stored_join;
pub mod trafficstar_pipes;
pub mod trafficstar_files;
pub mod trafficstar_networking;
pub mod trafficstar_proxy;
pub mod trafficstar_semaphore;
pub mod trafficstar_async_queue;
pub mod tempfiles;
pub mod trafficstar_dns_resolver;
pub mod trafficstar_async_semaphore;
pub mod state_locker;
pub mod shuffler;
pub mod randomizer;
pub mod sink;
mod test;
use std::{ mem::MaybeUninit, process::{ExitStatus, Output}, sync::{Arc, Once, OnceLock}};
use serde::Deserialize;
use tokio::runtime::{Runtime};
use trafficstar_errors::traffic_star_error::TrafficStarError;
#[derive(Deserialize)]
struct IpifyResponseFormat{
    pub ip : String
}
/// Returns public ip or 0.0.0.0 if failed.
#[allow(unsafe_code)]
pub fn fetch_public_ip() -> String
{
    
    static SINGLETON: OnceLock<String> = OnceLock::new();
    const IPIFY_URL : &str = "https://api.ipify.org/?format=json";
    SINGLETON.get_or_init(|| {
    let res = reqwest::blocking::get(IPIFY_URL);
    match res.and_then(reqwest::blocking::Response::json::<IpifyResponseFormat>) {
        Ok(val) => {
            val.ip
        }
        Err(_) => {
            "0.0.0.0".to_string()
        }
    }
}).clone()
}

#[allow(unsafe_code)]
pub async  fn async_fetch_public_ip() -> String
{
    
    
    const IPIFY_URL : &str = "https://api.ipify.org/?format=json";
    
    if let Ok(res) = reqwest::get(IPIFY_URL).await
        && let Ok(ip)= res.json::<IpifyResponseFormat>().await {
        
        ip.ip
    }else{
        "0.0.0.0".to_string()
    }
}

#[allow(unsafe_code,static_mut_refs)]
///Static behaviour required by logger.
pub fn get_singleton_multi() -> Arc<Runtime> {
    static mut SINGLETON: MaybeUninit<Arc<Runtime>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    // SAFETY:
    // Needed to create a singleton of an initializable as a static.
    unsafe {
        ONCE.call_once(|| {
            let runtime = Arc::new(tokio::runtime::Builder::new_multi_thread()
            .enable_all()   
            .build().unwrap());
            
          
            SINGLETON.write(runtime);

            
        });

        SINGLETON.assume_init_mut().clone()
    }
}

/// Gets multithreaded runtime with all features enabled!
pub fn get_multi_runtime() -> Result<Arc<Runtime>, std::io::Error>{
    Ok(Arc::new(tokio::runtime::Builder::new_multi_thread()
    .enable_all()   
    .build()?))
    //Ok(get_singleton_multi())
}

/// Gets multithreaded runtime with all features enabled!
pub fn get_owned_multi_runtime() -> Result<Runtime, std::io::Error>{
    tokio::runtime::Builder::new_multi_thread()
    .enable_all()   
    .build()
}
/// Creates singlethreaded runtime on local thread with all features enabled!
pub fn create_single_runtime() -> Result<Runtime, std::io::Error>{
    tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
}

pub fn get_public_interface() -> Result<default_net::Interface, std::io::Error> {
    match default_net::get_default_interface(){
        Ok(r) => {
            Ok(r)
        }
        Err(err) => {
            Err(std::io::Error::other(format!("Failed to get default interface, specified reason: {}!",err)))
        }
    }
}

pub fn get_public_interface_name() -> Result<String, std::io::Error> {
    Ok(get_public_interface()?.name)
}

#[allow(unsafe_code)]
pub fn get_public_interface_ipv4_str() -> Result<String, TrafficStarError> {
    static SINGLETON: OnceLock<Result<String,TrafficStarError>> = OnceLock::new();
    (*SINGLETON.get_or_init(|| {
        if let Some(ipv4) = get_public_interface()?.ipv4.into_iter().next(){
            return Ok(ipv4.addr.to_string())
        }
        Err(std::io::Error::other("Had no ipv4 fields set!").into())
    })).clone()
}

pub fn run_command(program : &str, args : Vec<&str>) -> Result<Output, TrafficStarError>{
    
    let mut process = std::process::Command::new(program);
    let mut string = program.to_string();
    for arg in args{
        process.arg(arg);
        string = string + " "+arg;
    }
    //println!("Running : {}",string);
    process.stdout(std::process::Stdio::piped());
    process.stderr(std::process::Stdio::piped());
    process.stdin(std::process::Stdio::piped());
    let child = process.spawn()?;
    let output = child.wait_with_output()?;
    if output.status != ExitStatus::default(){
        //warn!("Bad exit status, Error string : {}, command : {}",String::from_utf8(output.stderr.clone()).unwrap_or("None-utf8!".to_string()), string);
        Err(TrafficStarError::id_msg("BadExitStatus".into(), format!("Bad exit status, Error string : {}",String::from_utf8(output.stderr.clone()).unwrap_or("None-utf8!".to_string()))))
    }else{
        Ok(output)
    }
}


pub async fn async_run_command(program : &str, args : Vec<&str>) -> Result<Output, TrafficStarError>{
    
    let mut process = tokio::process::Command::new(program);
    let mut string = program.to_string();
    for arg in args{
        process.arg(arg);
        string = string + " "+arg;
    }
    //println!("Running : {}",string);
    process.stdout(std::process::Stdio::piped());
    process.stderr(std::process::Stdio::piped());
    process.stdin(std::process::Stdio::piped());
    let child = process.spawn()?;
    let output = child.wait_with_output().await?;
    if output.status != ExitStatus::default(){
        //warn!("Bad exit status, Error string : {}, command : {}",String::from_utf8(output.stderr.clone()).unwrap_or("None-utf8!".to_string()),string);
        Err(TrafficStarError::id_msg("BadExitStatus".into(), format!("Bad exit status, Error string : {}",String::from_utf8(output.stderr.clone()).unwrap_or("None-utf8!".to_string()))))
    }else{
        Ok(output)
    }
}
