#[cfg(test)]
#[allow(unused)]
mod tests {
    use std::{ io::{PipeWriter, Write}, os::fd::{AsRawFd, OwnedFd, RawFd}, sync::{Arc, Once}, time::Duration};


    use futures::{AsyncReadExt, AsyncWriteExt};
    use log::{debug, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::{panicerror, trafficstar_logger::TrafficStarLogger};

    use crate::{get_multi_runtime, fetch_public_ip, trafficstar_files::file_handler::FileHandler};


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

    fn file_reader_instance_setting_one(instance  : usize) -> 
   (std::thread::JoinHandle<PipeWriter>, 
   tokio::task::JoinHandle<Result<(),TrafficStarError>>){
            const MESSAGE : &str = "Howdy my friend, how is going today?";
            TrafficStarLogger::set_threadhook_nick(None);
            setup("FileReader".into());
            let (pipe_read, mut pipe_write) = std::io::pipe().unwrap();
            
           
            let reader = tokio::spawn(async move {
                let mut file_reader = FileHandler::new(pipe_read.into()).await.unwrap();
                let mut buffer : Vec<u8> = MESSAGE.as_bytes().to_vec();
                debug!("[{}], Ready when expected to be!", instance);
                let k = file_reader.read(&mut buffer).await.unwrap();
                debug!("[{}], Read {} bytes, {{{}}}",instance,k,String::from_utf8(buffer.clone()).unwrap());
                Ok(())
            });
             let writer = std::thread::spawn(move || {
                TrafficStarLogger::add_nick_thread("Writer".into());
                info!("Starting writer!");
                pipe_write.write_all(MESSAGE.as_bytes()).unwrap();
                pipe_write.flush().unwrap();
                info!("Wrote to pipe!");
                let _ = pipe_write.write_all("WOW".as_bytes());
                let _ = pipe_write.flush();
                info!("Written to pipe, closing!");
                pipe_write
            });

            (writer,reader)        
    }

    fn file_reader_instance_setting_two(instance  : usize) -> 
   (std::thread::JoinHandle<()>, 
   tokio::task::JoinHandle<Result<(),TrafficStarError>>){
            const MESSAGE : &str = "Howdy my friend, how is going today?";
            TrafficStarLogger::set_threadhook_nick(None);
            setup("FileReader".into());
            let (pipe_read, mut pipe_write) = std::io::pipe().unwrap();
            
           
            let reader = tokio::spawn(async move {
                let mut file_reader = FileHandler::new(pipe_read.into()).await.unwrap();
                let mut buffer : Vec<u8> = MESSAGE.as_bytes().to_vec();
                buffer.append(&mut MESSAGE.as_bytes().to_vec());
                debug!("[{}], Ready when expected to be!", instance);
                file_reader.read_exact(&mut buffer).await.unwrap();
                debug!("[{}], Read {} bytes, {{{}}}",instance,buffer.len(),String::from_utf8(buffer.clone()).unwrap());
                
                debug!("Seeing if it will close!");
                let k = file_reader.read(&mut buffer).await.unwrap();
                debug!("Read {} bytes!",k);
                assert_eq!(k,0,"Didn't read correct amount of bytes!");
                let k = file_reader.read(&mut buffer).await.unwrap();
                debug!("Read {} bytes!",k);
                assert_eq!(k,0,"Didn't read correct amount of bytes!");
                debug!("All done!");
                Ok(())
            });
             let writer = std::thread::spawn(move || {
                TrafficStarLogger::add_nick_thread("Writer".into());
                info!("Starting writer!");
                let _ = pipe_write.write_all(MESSAGE.as_bytes());
                std::thread::sleep(Duration::from_secs(5));
                let _ = pipe_write.write_all(MESSAGE.as_bytes());
                info!("Written to pipe, closing!");
                std::thread::sleep(Duration::from_secs(5));
                drop(pipe_write);
                info!("Dropped pipe!");
                
            });

            (writer,reader)        
    }
    
    async fn run_setting_one(){
        setup("FileReader".into());
        let mut tuples = Vec::new();
        info!("Creating tuples!");
        for i in 0..1{
            tuples.push(file_reader_instance_setting_one(i));
        }
        info!("Created tuples!");
        let mut parts = Vec::new();
        for tuple in tuples{
            let tuple= tuple;
            parts.push(tuple);
            
        }
        for part in parts{
            let _ = part.1.await.unwrap();
        }
    }

    async fn run_setting_two(){
        setup("FileReader".into());
        let mut tuples = Vec::new();
        info!("Creating tuples!");
        for i in 0..1{
            tuples.push(file_reader_instance_setting_two(i));
        }
        info!("Created tuples!");
        let mut parts = Vec::new();
        for tuple in tuples{
            let tuple= tuple;
            parts.push(tuple);
            
        }
        for part in parts{
            
            let _ = part.1.await.unwrap();
            info!("Joined like should!");
        }
    }
    
    #[tokio::test]
    async fn file_reader_test() {
       run_setting_two().await 
    }

    const TCPTESTPORT : u16 = 5201;

    async fn tcp_test_server(){
        debug!("Starting server!");
        let tcp_server = std::net::TcpListener::bind("0.0.0.0:".to_string()+&TCPTESTPORT.to_string()).unwrap();
        debug!("Started server!");
        let (tcp_stream, addr) = tcp_server.accept().unwrap();
        debug!("Accepted client!");
        let mut stream = FileHandler::new(tcp_stream.into()).await.unwrap();
        debug!("Created client FileHandler!");
        stream.write_all("Howdy!".as_bytes()).await.unwrap();
        debug!("Wrote all bytes!\nSleeping 10 secs");
        
        std::thread::sleep(Duration::from_secs(10));
        stream.write_all("Hi!".as_bytes()).await.unwrap();

    }

    async fn tcp_test_client(){
        let destination = fetch_public_ip()+":"+&TCPTESTPORT.to_string();
        info!("Binding to : {}",destination);
        let tcp_stream = match std::net::TcpStream::connect(fetch_public_ip()+":"+&TCPTESTPORT.to_string()){
            Ok(v) => v,
            Err(err) => panicerror!("{}",err),
        };
        let fd : OwnedFd = tcp_stream.into();
        debug!("Creating FileHandler for {}!",fd.try_clone().unwrap().as_raw_fd());
        let mut stream = FileHandler::new(fd).await.unwrap();
        let mut message = vec![0_u8;256];
        debug!("Reading for string!");
        let bytes = stream.read(&mut message).await.unwrap();
        debug!("read {} bytes",bytes);
        let bytes = stream.read(&mut message).await.unwrap();
        debug!("read {} bytes",bytes);
    }

    

    #[test]
    fn tcp_test() {
        fetch_public_ip();
        setup("TCP".into());
        let handle_server = std::thread::spawn(|| {
            TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec!["TCP".into(),"SERVER".into()] }));
            TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec!["TCP".into(),"SERVER".into()] }));
            let rt = get_multi_runtime().unwrap();
            rt.block_on(async {
                tcp_test_server().await;
            });

            // runtime drops here safely
        });

        std::thread::sleep(std::time::Duration::from_secs(1));

        let handle_client = std::thread::spawn(|| {
            TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec!["TCP".into(),"CLIENT".into()] }));
            TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec!["TCP".into(),"CLIENT".into()] }));
            let rt = get_multi_runtime().unwrap();
            rt.block_on(async {
                tcp_test_client().await;
            });
        });

        handle_client.join().unwrap();
        handle_server.join().unwrap();
    }


}

#[cfg(test)]
mod tests_tcp{
    use std::{io::Write, net::{SocketAddr, TcpListener, TcpStream}, sync::Once};

    use log::{error, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::TrafficStarLogger;

    use crate::{get_multi_runtime, trafficstar_files::file_handler::FileHandler, trafficstar_pipes::{TrafficStarPipePair, TrafficStarPipePairAsync}};

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

     fn tcp_client(addr : SocketAddr) -> Result<(), TrafficStarError>{
        let mut stream = TcpStream::connect(addr)?;
        info!("Connected to proxy server!");
        stream.write_all("Hi, hows it going my friend?\0".as_bytes())?;
        info!("Written to stream");
        drop(stream);
        Ok(())
    }

    async fn tcp_server(listener : TcpListener) -> Result<(), TrafficStarError>{
        let (stream,_addr) = listener.accept()?;
        let mut file_handle = FileHandler::new(stream.into()).await?;
        loop {
            match file_handle.read_string(Vec::with_capacity(256),None).await{
                Ok(v) => info!("Read string {}",v),
                Err(err) => {
                    if let Some(err) = err.get_ioerror()
                    && err.kind() == std::io::ErrorKind::UnexpectedEof{
                        info!("Got EOF as expected");
                        return Ok(())
                    }else{
                        return Err(err);
                    }
                },
            };
            
        }
    }

    #[test]
    fn tcp_test(){
        let tcp_listener = TcpListener::bind("0.0.0.0:0000").unwrap();
        let addr = tcp_listener.local_addr().unwrap();

        
        let client = std::thread::spawn(move || {
            setup("TcpClient".into());
            tcp_client(addr)
        });

        let server = std::thread::spawn(move || {
            let rt = get_multi_runtime().unwrap();
            let future = rt.spawn(async {
                setup("TcpServer".into());
                tcp_server(tcp_listener).await
            });
            rt.block_on(future).unwrap()
            
        });

        if let Err(client) = client.join().unwrap(){
            error!("Received error from tcp client : {}", client);
        }
        if let Err(server) = server.join().unwrap(){
            error!("Received error from tcp server : {}",server);
        }
    }

    fn async_pipes_client(mut stream : TrafficStarPipePair) -> Result<(),TrafficStarError>{
        let p = "Hi".to_string();
        stream.send(p)?;
        let p = "Howdy fellas".to_string();
        stream.send(p)?;
        Ok(())
    }

    #[allow(unused, clippy::unused_async)]
    async fn async_pipes_server(stream : TrafficStarPipePair) -> Result<(),TrafficStarError>{
        let mut stream = TrafficStarPipePairAsync::try_from(stream).await?;
        let string = stream.read::<String>().await?;
        info!("Read string {}!",string);
        let string = stream.read::<String>().await?;
        info!("Read string {}!",string);
        Ok(())
    }


    #[test]
    fn async_pipes(){
        for _i in 0..200{
            let pair = TrafficStarPipePair::new_pairs().unwrap();

            
            let client = std::thread::spawn(move || {
                setup("NoneAsyncPipeEnd".into());
                async_pipes_client(pair.0)
            });

            let server = std::thread::spawn(move || {
                let rt = get_multi_runtime().unwrap();
                let future = rt.spawn(async {
                    setup("AsyncPipeEnd".into());
                    async_pipes_server(pair.1).await
                });
                rt.block_on(future).unwrap()
                
            });

            if let Err(client) = client.join().unwrap(){
                error!("Received error from NoneAsync pipe : {}", client);
            }
            if let Err(server) = server.join().unwrap(){
                error!("Received error from Async Pipe : {}",server);
            }
        }
    }
}