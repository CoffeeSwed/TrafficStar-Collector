pub mod proxy;
mod proxy_slave;
mod proxy_commands;
#[allow(clippy::unused_async,unused)]
#[cfg(test)]
mod tests{
    /*use std::{io::{Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, sync::Once};

    use log::{error, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::TrafficStarLogger;

    use crate::{create_multi_runtime, trafficstar_proxy::proxy::HttpProxyServer};

    #[warn(unused_unsafe)]
    pub fn setup(test_name : Option<String>) {
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res {
                log::set_max_level(log::LevelFilter::Debug);
            }
        });
        if let Some(test_name) = test_name{
            TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name.clone()] }));
            TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name,"CHILD".into()] }));
        }
    }
    
    async fn proxy_client(addr : SocketAddr) -> Result<(), TrafficStarError>{
        const IPIFY_URL: &str = "https://api.ipify.org/";
        let client = reqwest::ClientBuilder::new().
            proxy(reqwest::Proxy::https(addr.to_string()).unwrap())
            .proxy(reqwest::Proxy::http(addr.to_string()).unwrap())
            .build().unwrap();
        let request = client.get(IPIFY_URL).build().unwrap();
        match client.execute(request).await{
            Ok(v) => {
                let ipaddr = String::from_utf8(v.bytes().await.unwrap().to_vec()).unwrap();
                info!("Fetched ip {}!",ipaddr);
                Ok(())
            },
            Err(err) => Err(TrafficStarError::msg(format!("{}",err))),
        }
    }

   

    #[test]
    fn proxy_test(){
        let tcp_listener = TcpListener::bind("0.0.0.0:0000").unwrap();
        let addr = tcp_listener.local_addr().unwrap();

        
        let client = std::thread::spawn(move || {
            let rt = create_multi_runtime()?;
            rt.block_on(async move {
                proxy_client(addr).await
            })
        });
        setup(None);
        let mut handler = HttpProxyServer::new(tcp_listener).unwrap();


        if let Err(client) = client.join().unwrap(){
            error!("Received error from proxy client : {}", client);
        }
        drop(handler);
    }*/
}