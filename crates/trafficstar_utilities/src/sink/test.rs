use std::{sync::Once, time::Duration};

use log::{debug, error};
use tokio::net::{TcpStream};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{trafficstar_logger::TrafficStarLogger};

use crate::{async_fetch_public_ip, get_multi_runtime, sink::{receiver::SinkReceiver, sender::SinkSender}};

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

#[tokio::test]
async fn sender_receiver_test(){

    let receiver = std::thread::spawn(move || {
        setup("Receiver".into());
        get_multi_runtime().unwrap().block_on(async move {
        receiver().await
    })});

    let sender = std::thread::spawn(move || {get_multi_runtime().unwrap().block_on(async move {
        setup("Sender".into());
        sender().await
    })});


    setup("SenderReceiverTest".into());
    if let Err(err) = sender.join().unwrap(){
        error!("Received error from sender :{}",err);
    }
     if let Err(err) = receiver.join().unwrap(){
        error!("Received error from reciver :{}",err);
    }
}

async fn receiver() -> Result<(),TrafficStarError>{
    let tcp_listener = tokio::net::TcpListener::bind("0.0.0.0:5201").await.unwrap();

    debug!("Waiting for client!");
    let (stream,_) = match tcp_listener.accept().await{
        Ok(v) => v,
        Err(err) => {
            error!("Got error accepting client : {}",err);
            return Err(err.into());
        },
    };
    debug!("Accepted client!");
    let stream = SinkReceiver::new(stream)?;
    for _i in 0..100{
        tokio::time::sleep(Duration::from_millis(100)).await;
        debug!("Read speed : {} ({} Mb/s)\n\tTransfer {} MiB",stream.get_speed(), stream.get_speed_mbits(),stream.get_transfered_mebibytes());
    }
    debug!("Waited done!");
    stream.kill().await;
    debug!("Killed!");
    Ok(())
}


async fn sender() -> Result<(),TrafficStarError>{
    
    let tcp_socket = TcpStream::connect(async_fetch_public_ip().await+":5201").await?;
    debug!("Connected!");
    let stream = SinkSender::new(tcp_socket)?;
    tokio::time::sleep(Duration::from_secs(20)).await;
    stream.kill().await?;
    Ok(())
}