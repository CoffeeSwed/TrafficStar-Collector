use std::{path::PathBuf, process::{Command, Output}};
use trafficstar_errors::traffic_star_error::TrafficStarError;

#[allow(dead_code)]
pub fn start_wg_quick(path : PathBuf) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("wg-quick");
    command.arg("up")
    .arg(path.to_str().unwrap());
    let mut process = Proccesses::create_process(&mut command, "wg_process_start".to_string())?;
    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}

#[allow(dead_code)]
pub fn stop_wg_quick(path : PathBuf) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("wg-quick");
    command.arg("down")
    .arg(path.to_str().unwrap());
    let mut process = Proccesses::create_process(&mut command, "wg_process_stop".to_string())?;
    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}

#[allow(dead_code)]
pub fn delete_interface(interface : String) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("ip");
    command.arg("link").arg("delete")
    .arg(&interface);
    let mut process = Proccesses::create_process(&mut command, "ip_link_delete".to_string())?;

    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}

pub fn route_wg(interface_name : String, fwmark : String) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("wg");
    
    command.arg("set")
    .arg(interface_name)
    .arg("fwmark")
    .arg(fwmark);
    let mut process = Proccesses::create_process(&mut command, "wg_process_route".to_string())?;
    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}

pub fn stop_tso(interface_name : String) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("ethtool");
    
    command.arg("-K")
    .arg(interface_name)
    .arg("tso")
    .arg("off");
    let mut process = Proccesses::create_process(&mut command, "wg_tso_off".to_string())?;
    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}

pub fn start_tso(interface_name : String) -> Result<Output, TrafficStarError>{
    let mut command: Command = Command::new("ethtool");
    
    command.arg("-K")
    .arg(interface_name)
    .arg("tso")
    .arg("on");
    let mut process = Proccesses::create_process(&mut command, "wg_tso_on".to_string())?;
    let _res = Proccesses::wait_with_output(&mut process);
    if let Ok(res) = _res{
        Ok(res)
    }else{
        Err(format!("{}",_res.err().unwrap()).into())
    }
}