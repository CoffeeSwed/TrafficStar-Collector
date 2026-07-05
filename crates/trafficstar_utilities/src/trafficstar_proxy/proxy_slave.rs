use std::{net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener}, str::FromStr, sync::Arc, time::Duration};

use futures::FutureExt;
use hickory_resolver::config::ResolverConfig;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::{Mutex, Notify, RwLock}};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{sdebug, serror, sinfo, swarn};
use trafficstar_logger_macro::StructLoggerName;

use crate::{trafficstar_dns_resolver::DnsResolver, trafficstar_pipes::{TrafficStarPipePairAsync}, trafficstar_proxy::{proxy::HttpProxyRequest, proxy_commands::HttpProxyCommands}};

#[derive(StructLoggerName)]
pub struct HttpProxySlave{
    listener : TcpListener,
    command_channel : Arc<Mutex<TrafficStarPipePairAsync>>,
    resolver : Arc<DnsResolver>,
    buffer_remote_host : Option<(bool,Duration)>
}

impl HttpProxySlave{
    pub async fn start(listener : TcpListener, 
        command_channel : TrafficStarPipePairAsync,
        buffer_remote_host : Option<(bool,Duration)>) -> Result<(),TrafficStarError>{
        
        
        Self{
            listener,
            command_channel : Arc::new(Mutex::new(command_channel)),
            resolver : Arc::new(DnsResolver::new(ResolverConfig::cloudflare())),
            buffer_remote_host
        }.run().await
    }
    
    async fn run(self) -> Result<(), TrafficStarError>{
        self.listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(self.listener)?;
        sdebug!("Created FileHandler And Tokio TcpListener!");
        
        let notify_stop = Arc::new(Mutex::new(Arc::new(Notify::new())));
        let communication = self.command_channel.lock().await;
        let mut future_read = communication.read::<HttpProxyCommands>();
        
        
        let entry_lock = Arc::new(RwLock::new(0_u8));


        let entry_lock_clone = entry_lock.clone();
        let notify_stop_clone = notify_stop.clone();
        let buffer_output_clone = self.buffer_remote_host;
        let listen_task = tokio::task::spawn(async move {
        loop{
            let holders_clone = entry_lock_clone.clone();
                if let Ok(part) = listener.accept().await{
                    let addr = part.1;
                    sdebug!("Accepted client {}!",addr);
                    let stream = part.0;

                    let lock = notify_stop_clone.lock().await;
                    let rwlock = holders_clone.read_owned().await;
                    

                    let notify_stop_clone = (*lock).clone().notified_owned().shared();
                    let resolver = self.resolver.clone();
                    tokio::spawn(async move {
                        tokio::select! {
                            _ = notify_stop_clone => {
                                sdebug!("Received stop!")
                            },
                            result = Self::handle_client(stream, resolver, buffer_output_clone) => {
                                 if let Err(result) = result{
                                        serror!("Client handler returned with error : {}",result);
                                    }else{
                                        sdebug!("Finished with a client!");
                                    }
                            }
                        }
                       
                        drop(rwlock);
                    });
                    drop(lock);
                }
            }
        });
        communication.send(HttpProxyCommands::Startedslave).await?;

        loop{
            match future_read.await{
                Ok(v) => {
                    match v{
                        HttpProxyCommands::Restart => {
                            sdebug!("Killing children and restarting!");
                            let lock = notify_stop.lock().await;
                            lock.notify_waiters();
                            sdebug!("Notified children!");
                            drop(lock);
                            sdebug!("Dropped lock!");
                            let entry_lock = entry_lock.write().await;
                            drop(entry_lock);

                            sdebug!("Killed children!");

                            communication.send(HttpProxyCommands::Restart).await?;
                            
                        },
                        HttpProxyCommands::Stop => {
                            sinfo!("Received kill command!");
                            break;
                        },
                        _v =>
                        {
                            serror!("Received invalid command!");
                            break;
                        },
                    }
                },
                Err(err) => {
                    serror!("Received error when reading for commands : {}",err);
                    break;
                }
            };
            future_read = communication.read::<HttpProxyCommands>();
            
        }
        sinfo!("Sending kill to children!");
        listen_task.abort();
        let _ = listen_task.await;
        notify_stop.lock().await.notify_waiters();
        drop(entry_lock.write().await);
        communication.send(HttpProxyCommands::Stopped).await?;
        Ok(())
    }

    async fn get_header(stream : &mut tokio::net::TcpStream) -> Result<String,TrafficStarError>{
        let mut buffer : Vec<u8> = vec![0; 1];
        let mut header_string = String::new();
        
        while !header_string.ends_with("\r\n\r\n") && !header_string.ends_with("\n\n") {
            if let Err(err) = tokio::io::AsyncReadExt::read_exact(stream, &mut buffer).await {
                return Err(err.into())
            } else {
                let string = String::from_utf8_lossy(&buffer);
                header_string = header_string + &string;
            }
        }
        Ok(header_string)
    }
    
    #[allow(unused,dead_code,clippy::unused_async)]
    async fn handle_client(stream_tokio : tokio::net::TcpStream,
    resolver : Arc<DnsResolver>,
    buffer_remote_host : Option<(bool,Duration)>
    ) -> Result<(),TrafficStarError>{
        let mut buf_client = vec![0_u8;8192];
        let mut buf_server = vec![0_u8;8192];
        let mut stream_client = stream_tokio;
        
        let header = match Self::get_header(&mut stream_client).await{
            Ok(v) => v,
            Err(err) => {
                swarn!("Received bad header, error : {}!",err);
                return Ok(())
            },
        };
        
        let host = HttpProxyRequest::find_dest(&header)?;
        let port = HttpProxyRequest::find_port(&header)?;
        let ipv4_addr = match Ipv4Addr::from_str(&host){
            Ok(v) => v,
            Err(_) => {
                resolver.resolve_ipv4(&host).await?
            },
        };
        sdebug!("Connecting to end host : {} ({}:{})",host,ipv4_addr,port);
        let mut stream_dest =
            tokio::net::TcpStream::connect(SocketAddr::new(IpAddr::V4(ipv4_addr),port)).await?;
        sdebug!("Connected to end host : {} ({}:{})",host,ipv4_addr,port);
        let response = HttpProxyRequest::msg(header.clone());
        if HttpProxyRequest::is_https(&header){
            stream_client.write_all(response.as_bytes()).await?;
            stream_client.flush().await?;
        }else{
            stream_dest.write_all(response.as_bytes()).await?;
            stream_dest.flush().await?;   
        }
        loop{
            tokio::select! {
                 v = stream_dest.read(&mut buf_server) => {
                    match v{
                        Ok(v) => {
                            if v == 0{
                                return Ok(())
                            }
                            stream_client.write_all(&buf_server[..v]).await?;
                            stream_client.flush().await?;
                        }
                        Err(v) => {
                            return Err(v.into())
                        }
                    }
                },
                v = stream_client.read(&mut buf_client) => {
                    match v{
                        Ok(v) => {
                            if v == 0{
                                return Ok(())
                            }
                            stream_dest.write_all(&buf_client[..v]).await?;
                            stream_dest.flush().await?;
                        }
                        Err(v) => {
                            return Err(v.into())
                        }
                    }
                },
        }
    }

    }
}
