//Compile-time variables
use std::{net::SocketAddr, sync::Arc, time::Duration};


use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{net::TcpStream, task::JoinHandle, time::Instant};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{serror, swarn};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{get_singleton_multi, trafficstar_async_queue::AsyncSharedQueue, trafficstar_files::{file_async_rpm::FileAsyncRpm, file_handler::FileHandler}};



#[derive(StructLoggerName)]
pub struct Connection {
    pub peer_addr : SocketAddr,
    pub read_packets : Arc<AsyncSharedQueue<Vec<u8>>>,
    pub write_packets : Arc<AsyncSharedQueue<Vec<u8>>>,
    pub read_writer_task : Option<JoinHandle<()>>
}



pub async fn create_connection_struct(stream: TcpStream) -> Result<Connection,TrafficStarError> {
    let peer_addr = match stream.peer_addr(){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::id_msg("SockAddr".into(), format!("{}",err))),
        };
   
    let write_packets: Arc<AsyncSharedQueue<Vec<u8>>> = AsyncSharedQueue::new().into();
    let read_packets: Arc<AsyncSharedQueue<Vec<u8>>> = AsyncSharedQueue::new().into();

    
    let write_packets_clone = write_packets.clone();
    let read_packets_clone = read_packets.clone();

    let read_writer_task = tokio::spawn(async move{
        Connection::read_writer_run(stream, write_packets_clone, read_packets_clone).await
    });
    
    let res = Connection {  
        peer_addr,
        read_packets,
        write_packets,
        read_writer_task : Some(read_writer_task)
    };

    Ok(res)
}


#[derive(Serialize,Deserialize)]
enum ConnectionHeaders{
    Keepalive,
    ConnectionFrame{
        last_frame : bool,
        data : Vec<u8>
    }
}



const TIMEOUT_READ : Duration = Duration::from_secs(20);
const TIMEOUT_WRITE : Duration = Duration::from_secs(10);
const FRAME_MAX_SIZE : usize = 8192;

impl Connection{
    pub async fn read<T : DeserializeOwned>(&mut self) -> Result<T, TrafficStarError> {
        let packet = self.read_packets.pop().await?;
        match rmp_serde::decode::from_slice::<T>(&packet){
            Ok(v) => Ok(v),
            Err(err) => Err(TrafficStarError::id_msg("rmp_serde::decode::from_slice".into(),format!("{}",err))),
        }
    }

    pub async fn send<T : Serialize>(&mut self, data : T) -> Result<(), TrafficStarError> {
       let data = match rmp_serde::encode::to_vec(&data){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::id_msg("rmp_serde::encode::to_vec".into(),format!("{}",err))),
        };
        self.write_packets.push(data);
        Ok(())
    }

    async fn read_writer_run(connection : tokio::net::TcpStream, 
        write_packets : Arc<AsyncSharedQueue<Vec<u8>>>, 
        read_packets : Arc<AsyncSharedQueue<Vec<u8>>>){
        
        let connection = match FileHandler::new(match connection.into_std(){
            Ok(v) => v,
            Err(err) => {serror!("Failure creating tokio stream into regular one : {}",err); return},
        }.into()).await{
            Ok(v) => v,
            Err(err) => {
                serror!("Error creating filehandler : {}",err);
                write_packets.close();
                read_packets.close();
                return;
            },
        };
        let mut next_read_timeout = match Instant::now().checked_add(TIMEOUT_READ){
            Some(v) => v,
            None => {write_packets.close();
                    read_packets.close();
                    return;
            }
        };
        let mut next_write_timeout = match Instant::now().checked_add(TIMEOUT_WRITE){
            Some(v) => v,
            None => {write_packets.close();
                    read_packets.close();
                    return;
            }
        };

        let read_packets_internal_queue = Arc::new(AsyncSharedQueue::<ConnectionHeaders>::new());
        



        let read_packet_tasks = 
        {
        //let read_packets = read_packets.clone();
        let read_packets_internal_queue = read_packets_internal_queue.clone();
        let connection = FileAsyncRpm::new(connection.clone());
            tokio::task::spawn(async move{
                loop{
                    match connection.read::<ConnectionHeaders>().await{
                        Ok(v) => {
                            read_packets_internal_queue.push(v);
                        },
                        Err(err) => {
                            swarn!("Error reading from stream, err : {}. Closed?",err);
                            read_packets_internal_queue.close();
                            break;
                        },
                    }
                }
            })
        };

        let mut buffer_read = Vec::<u8>::new();
        let connection = Arc::new(FileAsyncRpm::new(connection.clone()));

        let write_packets = write_packets.clone();
        let read_packets = read_packets.clone();
        let connection = connection.clone();

        loop {

            let (update_read, update_write) = 

                tokio::select! {
                _ = tokio::time::sleep_until(next_write_timeout) => {
                    match tokio::time::timeout(TIMEOUT_WRITE,async{
                        connection.send(ConnectionHeaders::Keepalive).await
                    }).await{
                        Ok(v) => {
                            if let Err(err) = v{
                                serror!("Received error : {}",err);
                                break;
                            }
                            (false,true)
                        },
                        Err(_) => {
                            serror!("Timeout reached sending keep_alive, ({:?})",TIMEOUT_WRITE);
                            break;
                        },
                    }
                },
                packet = read_packets_internal_queue.pop() => {
                    match packet{
                        Ok(packet) => {
                            match packet{
                                ConnectionHeaders::ConnectionFrame{ last_frame, mut data } => {
                                    buffer_read.append(&mut data);
                                    if last_frame{
                                        read_packets.push(buffer_read);
                                        buffer_read = Vec::new();
                                    }
                                    (true,false)
                                },
                                _ => {
                                    (true,false)
                                },
                            }
                        },
                        Err(err) => {
                            serror!("Received read error : {}",err);
                            break;
                        },
                    }
                },
                packet = write_packets.pop() => {
                    match packet{
                        Ok(mut packet) => {

                            while !packet.is_empty(){
                                let sending = if packet.len() <= FRAME_MAX_SIZE{
                                    let res = ConnectionHeaders::ConnectionFrame{
                                        data : packet.clone(),
                                        last_frame : true
                                    };
                                    packet.clear();
                                    res
                                }else{
                                    let new_vec = packet.split_off(FRAME_MAX_SIZE);
                                    let res = ConnectionHeaders::ConnectionFrame{
                                        data : packet,
                                        last_frame : false
                                    };
                                    packet = new_vec;
                                    res
                                };
                                let connection = connection.clone();
                                if let Err(err) = match tokio::time::timeout(TIMEOUT_WRITE,async move{
                                        connection.send(sending).await
                                }).await{
                                    Ok(v) => {
                                        v
                                    },
                                    Err(_) => {
                                        Err(format!("Timeout reached! {:?}",TIMEOUT_WRITE).into())
                                    },
                                }{
                                    serror!("Error sending data : {}",err);
                                    write_packets.close();
                                    read_packets.close();
                                    break;
                                }
                            }
                            (true,true)
                        },
                        Err(err) => {
                            serror!("Received error reading packet : {}",err);
                            break;
                        },
                    }
                    
                }
                _ = tokio::time::sleep_until(next_read_timeout) => {
                    serror!("Read timeout!");
                    break;
                },
             };
            

            if update_read{
                next_read_timeout = match Instant::now().checked_add(TIMEOUT_READ){
                    Some(v) => v,
                    None => {
                        break;
                    }
                };
            }
            if update_write{
                next_write_timeout = match Instant::now().checked_add(TIMEOUT_WRITE){
                Some(v) => v,
                    None => {
                        break;
                    }
                };
            }
        }
        
        read_packet_tasks.abort();
        let _ = read_packet_tasks.await;
        write_packets.close();
        read_packets.close();
    
    }
}

impl Drop for Connection{
    fn drop(&mut self) {
        let task = self.read_writer_task.take();
        if let Some(task) = task{
            get_singleton_multi().spawn(async move{
                task.abort_handle().abort();
                let _ = task.await;
            });
        }
    }
}