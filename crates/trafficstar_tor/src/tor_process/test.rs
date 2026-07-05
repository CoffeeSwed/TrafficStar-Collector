#[cfg(test)]
#[allow(dead_code, unused)]
mod tests
{
    const TOR_DEVICE_TEST_PORT : u16 = 5201;
    const TOR_DEVICE_TEST_MESSAGE : &str = "Hi friend, hows it going today?";

    use std::{io::{ErrorKind, Read, Write}, net::SocketAddr, str::FromStr, sync::Once, thread::JoinHandle, time::Duration};

    use log::{error, info, warn};
    use socket2::{Domain, Protocol, Socket, Type};
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpSocket}};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_interface::reservation::{InterfaceReservation, Ipv4Reservation, MarkReservation};
    use trafficstar_logger::{panicerror, trafficstar_logger::TrafficStarLogger};
    use trafficstar_utilities::{async_fetch_public_ip, get_multi_runtime, get_singleton_multi, trafficstar_files::{file_async_rpm::FileAsyncRpm, file_handler::FileHandler}, trafficstar_networking::interface::TrafficStarInterfaceName};

    use crate::tor_process::{TorProcess, TorInterfaceConfig};

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

    

    async fn tor_server(port : u16) -> Result<(),TrafficStarError>{
        let (listener, client) = TcpListener::bind("0.0.0.0:".to_string()+&port.to_string()).await?.accept().await?;
        let asyncfile = FileAsyncRpm::new(FileHandler::new(listener.into_std()?.into()).await?);

        loop {
            match asyncfile.read::<String>().await {
                Ok(msg) => {
                    info!("{}",msg);
                    asyncfile.send(msg).await?;
                },
                Err(err) => {
                    if let Some(io) = err.get_ioerror() 
                    && io.kind() == ErrorKind::UnexpectedEof{
                        info!("Got EOF");
                        break
                    }
                    return Err(err);
                }
            }
        }
        
        Ok(())
    }
    
    async fn tor_client(port : u16) -> Result<(),TrafficStarError>{
        const MESSAGE_LOOPS : usize = 16;
        let config = TorInterfaceConfig::new(TrafficStarInterfaceName::from_str("eth0")?).await;
        let device = TorProcess::new(config).await?;
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.bind_device(Some(device.name().as_array()))?;
        let socketaddr = match SocketAddr::from_str(&(async_fetch_public_ip().await+":"+&port.to_string())){
            Ok(v) => v,
            Err(err) => return Err(format!("Parse error : {}",err).into())
        };
        let res = tokio::task::spawn_blocking(move || {
            socket.connect(&socketaddr.into())?;
            Ok::<socket2::Socket,TrafficStarError>(socket)
        }).await.unwrap();
        let stream : std::net::TcpStream = res?.into();
        let asyncfile = FileAsyncRpm::new(FileHandler::new(stream.into()).await?);
        let message = "I once met a pretty laddy, it was pretty cool!".to_string();
        for _i in 0..MESSAGE_LOOPS{
            asyncfile.send(message.clone()).await?;
            info!("Sent message!");
        }
        for _i in 0..MESSAGE_LOOPS{
            let read = asyncfile.read::<String>().await?;
            if read == message{
                info!("Read message Back");
            }else{
                return Err(format!("Read {} instead of {}",read,message).into())
            }
        }
        drop(device);
        
        Ok(())
    }
      
    #[test]
    fn tor_process_test(){
        get_singleton_multi();
        setup("TorDeviceTest".into());
        info!("Starting tor device test!");
        for port in 5201..5202{
            let receiver = 
            std::thread::spawn(move || {
                setup("Receiver".into());

                get_multi_runtime().unwrap().block_on({
                    tor_server(port)
                })
            });
            

            let tor_client_task = 
            std::thread::spawn(move || {
            setup("Sender".into());
            get_multi_runtime().unwrap().block_on(tor_client(port))
            });
            
            if let Err(err) = tor_client_task.join().unwrap(){
                error!("Error client : {}",err);
            }

            if let Err(err) = receiver.join().unwrap(){
                error!("Error receiver : {}",err);
            }
        }
    } 


    
}

