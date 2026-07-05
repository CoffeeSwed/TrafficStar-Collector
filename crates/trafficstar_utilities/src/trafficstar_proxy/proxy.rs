use std::{net::TcpListener, sync::Arc, time::Duration};

use colored::Color;
use tokio::sync::Mutex;
use trafficstar_errors::{traffic_star_error::TrafficStarError, trafficstar_error_traits::TrafficStarEnumErrorTrait};
use trafficstar_logger::{serror, sinfo, trafficstar_logger::TrafficStarLogger};
use trafficstar_logger_macro::StructLoggerName;
use url::Url;

use crate::{get_multi_runtime, get_singleton_multi, trafficstar_pipes::TrafficStarPipePairAsync, trafficstar_proxy::{proxy_commands::HttpProxyCommands, proxy_slave::HttpProxySlave}};
use trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName;

pub struct HttpProxyRequest{
    pub request : String
}

#[allow(dead_code)]
impl HttpProxyRequest{
    pub fn first_line(header : &str) -> &str{
        if let Some(v) = header.find(['\r', '\n']){
                    &header[0..v]
        }else{
            header
        }
    }
    
    pub fn is_https(header : &str) -> bool{
        Self::first_line(header).to_ascii_lowercase().contains("connect ")
    }

    pub fn find_url_string(header : &str) -> Option<String>{
        if let Some(start) =Self::first_line(header).find(" ")
            && let Some(end) = header[start+1..].find(" "){
                let end = start + 1 + end;
                return Some(header[start + 1..end].to_string())
            }
        None
    }

    pub fn find_url(header : &str) -> Option<Url>{
        if let Some(part) =Self::find_url_string(header) &&
            let Ok(url) = Url::parse(&part){
                Some(url)
            
        }else{
            None
        }
    }

    pub fn find_host(header : &str) -> Result<String,TrafficStarError>{
        if Self::is_https(header){
                if let Some(v) = header.to_ascii_lowercase().find("host: "){
                    let part = &header[(v+"Host: ".len())..];
                    if let Some(host) = part.find(['\r', '\n']).map(|v| part[0..v].to_string()){
                        return if host.contains(":"){
                            Ok(host)
                        }else{
                            Ok(host+ ":443")
                        }
                    }
                
            }
        }else {
            if let Some(url) = Self::find_url(header) &&
                 let Some(host) = url.host_str() &&
                    let Some(port) = url.port_or_known_default(){
                        return Ok(host.to_string()+":"+&port.to_string())
                    
                }
        }
        Err(TrafficStarError::msg("Host not detected!".to_string()))

    }

    pub fn find_dest(header : &str) -> Result<String, TrafficStarError>{
        let binding = Self::find_host(header)?;
        let host : Vec<&str> = binding.split(':').collect();
        if host.len() == 2{
            
            Ok(host[0].trim_end_matches(".").to_string())
        }else{
            Err(TrafficStarError::msg(format!("Host contains {} ':' characters, expected 1", host.len()-1)))
        }
    }


    pub fn find_port(header : &str) -> Result<u16, TrafficStarError>{
        let binding = Self::find_host(header)?;
        let host : Vec<&str> = binding.split(':').collect();
        if host.len() == 2{
            match host[1].parse::<u16>(){
                Ok(v) => Ok(v),
                Err(err) => Err(format!("Failed to parse {}, error : {}",host[1],err).into()),
            }
        }else{
            Err(TrafficStarError::msg(format!("Host contains {} ':' characters, expected 1", host.len()-1)))
        }
    }

    pub fn new_path(header : &str) -> Option<String>{
        if !Self::is_https(header) &&
            let Some(url) = Self::find_url(header){
                let mut res = url.path().to_string();
                if let Some(queries) = url.query(){
                    res = res + "?"+queries;
                }
                if let Some(fragment) = url.fragment(){
                    res = res + "#"+fragment;
                }
                return Some(res)
        }
        None
    }


    pub fn msg(header_string : String) -> String{
        let mut message = match Self::is_https(&header_string){
            true => "HTTP/1.1 200 Connection Established\r\nProxy-Agent: TrafficStarTorProxy/1.0\r\n\r\n".to_string(),
            false => header_string.clone(),
        };
        if !Self::is_https(&header_string){
            let first_line = Self::first_line(&header_string).to_string();
            if let Some(old) = Self::find_url_string(&header_string)
                && let Some(new) = Self::new_path(&header_string){
                                message = first_line.replace(&old, &(" ".to_string()+&new)) + &message[Self::first_line(&message).len()..];
                }
        }
        message
    }

}

#[derive(StructLoggerName)]
pub struct HttpProxyServer{
    command_channel : Option<Arc<Mutex<TrafficStarPipePairAsync>>>,
}

impl HttpProxyServer{
    pub async fn new(listener : TcpListener, buffer_remote_host : Option<(bool,Duration)>) -> Result<HttpProxyServer,TrafficStarError>
    {
        TrafficStarLogger::set_target_color(Self::struct_name(), Some(Color::Black));
        let (our_channel, their_channel) = TrafficStarPipePairAsync::new_pairs().await?;
        std::thread::spawn(move || {
            sinfo!("Starting HttpProxyServer!");
            let rt = get_multi_runtime().unwrap();

            let future = rt.spawn(async move {
                if let Err(err) = HttpProxySlave::start(listener,their_channel, buffer_remote_host).await{
                    
                    serror!("HttpProxyError : {}",err)
                }
                sinfo!("ProxyServer finished!");
            });
            rt.block_on(future).unwrap();
            
        });
        
        match our_channel.read::<HttpProxyCommands>().await?{
            HttpProxyCommands::Startedslave => Ok(Self{
                command_channel : Some(Arc::new(Mutex::new(our_channel)))
            }),
            v => {
                Err(TrafficStarError::msg(format!("Slave didn't send msg : {}",v.enum_variant()))) 
            }
        }
    }

    pub async fn restart(&self) -> Result<(),TrafficStarError>{
        if let Some(command_channel) = self.command_channel.clone() 
        && let command_channel = command_channel.lock().await{
            command_channel.send(HttpProxyCommands::Restart).await?;
            if command_channel.read::<HttpProxyCommands>().await? != HttpProxyCommands::Restart{
                return Err("Slave responded with incorrect response!".into())
            }
            Ok(())
        }else{
            Err("Server seems to be killed?".into())
        }
    }

    pub async fn kill(&mut self) -> Result<(),TrafficStarError>{
        if let Some(channel) = self.command_channel.take() && let channel = channel.lock().await{
             if channel.send(HttpProxyCommands::Stop).await.is_ok() && channel.read::<HttpProxyCommands>().await? == HttpProxyCommands::Stopped{
                Ok(())
             }else{
                Err("Unexpected communication pattern received when performing kill!".into())
             }

        }else{
            Err("Server already killed?".into())
        }
    }


}

impl Drop for HttpProxyServer{
    fn drop(&mut self) {
        sinfo!("Dropping Http Proxy Server!");
        if let Some(command_channel) = self.command_channel.clone(){
            get_singleton_multi().spawn(async move {
                let command_channel = command_channel.lock().await;
                if command_channel.send(HttpProxyCommands::Stop).await.is_ok(){
                    sinfo!("Sent command!");
                    match command_channel.read::<HttpProxyCommands>().await{
                        Ok(v) => {
                            if v == HttpProxyCommands::Stopped{
                                sinfo!("Child stopped!");
                            }else{
                                serror!("Child sent incorrect command!");
                            }
                        },
                        Err(err) => {
                            serror!("Child didn't tell us it stopped! Err : {}",err);
                        },
                    }
                    
                }else{
                    serror!("Couldn't sent command to stop!");
                }
            });
        }
    }
}