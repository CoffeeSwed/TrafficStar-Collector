pub mod file_handler;
pub mod file_listener;
pub mod file_signals;
pub mod file_async_rpm;
mod file_listener_slave;
mod epoll_events;

#[cfg(test)]
mod tests{
    use std::{net::{TcpListener, TcpStream}, sync::Once, time::Duration};

    use log::{error, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::TrafficStarLogger;

    use crate::{create_single_runtime, fetch_public_ip, get_singleton_multi, trafficstar_files::{file_async_rpm::FileAsyncRpm, file_handler::FileHandler}};


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
    
    async fn pipe_receiver(pipe : FileAsyncRpm) -> Result<(),TrafficStarError>{
        tokio::time::sleep(Duration::from_secs(5)).await;
        while let Ok(message) = pipe.read::<String>().await{
            info!("Read \n\t{}!",message);
        }
        Ok(())
    }

    async fn pipe_sender(pipe : FileAsyncRpm) -> Result<(),TrafficStarError>{
        let message = "Howdy my friend, i was once were unfortantly pretty horrified to find that my dishwasher wasn't as good as i thought at making my dishes clean";
        for _i in 0..16384{
            pipe.send(message.to_string()).await?;
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn filehandler_test(){
        setup("FileHandlerTest".into());
        info!("Starting tests!");
        get_singleton_multi();
        let listener = TcpListener::bind("0.0.0.0:5201").unwrap();
        let client = std::thread::spawn(move || {
            setup("Receiver".into());
            let pipe_recv: TcpStream = TcpStream::connect(fetch_public_ip()+":5201").unwrap();

            if let Err(err) = create_single_runtime().unwrap().block_on(async move{
                let filehandler = FileHandler::new(pipe_recv.into()).await.unwrap();
                pipe_receiver(FileAsyncRpm::new(filehandler)).await
            }){
                error!("Error : {}",err);
            }
        });

        let sender = std::thread::spawn(move || {
            setup("Sender".into());

            let pipe_send = listener.accept().unwrap();
            let pipe_send = pipe_send.0;
            let filehandler = FileHandler::new(pipe_send.into());
            if let Err(err) = create_single_runtime().unwrap().block_on(async move{
                pipe_sender(FileAsyncRpm::new(filehandler.await.unwrap())).await
            }){
                error!("Error : {}",err);
            }
        });

        client.join().unwrap();
        sender.join().unwrap();

    }
}