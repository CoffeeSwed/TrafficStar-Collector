#[allow(unused)]
#[cfg(test)]
mod test{
    use std::{io::Write, net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream}, str::FromStr, sync::Once};

    use futures::{AsyncReadExt, AsyncWriteExt};
    use log::{error, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::TrafficStarLogger;

    use crate::{get_multi_runtime, run_command, trafficstar_files::file_handler::FileHandler, trafficstar_networking::{create_tunnel}};

    #[warn(unused_unsafe)]
    pub fn setup(test_name : String) {
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res {
                log::set_max_level(log::LevelFilter::Debug);
            }
        });
        TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name.clone()] }));
        TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name,"CHILD".into()] }));
        
    }

    #[test]
    fn create_tunnel_test(){
        setup("CreateInterface".into());
        match create_tunnel(&crate::trafficstar_networking::interface::TrafficStarInterfaceName::from_str("test-tor").unwrap(), Some(Ipv4Addr::from_str("169.254.42.1").unwrap()), Some(Ipv4Addr::from_str("255.255.255.0").unwrap())){
            Ok(_) => {
                info!("Created interface like expected!");
                info!("ip link show : \n{}",String::from_utf8(
                run_command("ip",vec!["link","show"]).unwrap().stdout).unwrap());
                info!("ip brief : \n{}",String::from_utf8(
                run_command("ip",vec!["--brief","a"]).unwrap().stdout).unwrap());
            },
            Err(err) => {
                error!("Received error : {}",err);
            },
        };
    }

    
}