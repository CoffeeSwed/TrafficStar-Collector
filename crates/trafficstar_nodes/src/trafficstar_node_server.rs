use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{net::TcpListener, sync::Mutex};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;
use trafficstar_connections::trafficstar_data_traversel_types::ConnectionType;
use trafficstar_connections::trafficstar_enums::{SinkHandShakeType, ProxyHandShakeType};
use trafficstar_connections::{
 trafficstar_connection::Connection,
    trafficstar_enums::HandshakeType,
    trafficstar_data_route::DataRoute as DataRoute
};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{sdebug, serror, sinfo};
use trafficstar_logger::trafficstar_logger::TrafficStarLogger;
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::sink::receiver::SinkReceiver;
use trafficstar_utilities::{get_multi_runtime, get_singleton_multi};
use trafficstar_utilities::state_locker::StateLocker;
use trafficstar_utilities::trafficstar_proxy::proxy::HttpProxyServer;
use url::Url;

use crate::trafficstar_nodes::{create_file_for_tcpdump_output};
use crate::{
    trafficstar_nodes::GeneralNodeData
};
use trafficstar_connections::trafficstar_connection::create_connection_struct;


use trafficstar_processes::trafficstar_tcpdump::ASyncTCPDump;
static USED_PORTS: Mutex<Vec<u16>> = Mutex::new(Vec::new());
struct ServerClientEntry {
    connection: Connection,
    their_interfaces: Vec<ConnectionType>,
    data_directory : PathBuf,
    rw_lock : Arc<StateLocker<u8>>,
    reported_route_info : Option<String>
}
#[derive(StructLoggerName)]
pub struct ServerNode {
    listener: TcpListener,
    general_data: GeneralNodeData,
    data_directory : PathBuf,
    running_guard : Arc<StateLocker<u8>>
}

impl ServerNode{
    pub fn init_server( the_listener: TcpListener,
    the_route: DataRoute,
    data_directory : PathBuf) -> Self{
        Self { listener: the_listener, general_data: GeneralNodeData { route: the_route }, data_directory, running_guard : Arc::new(StateLocker::new(0)) }
    }

    
    pub fn run_server(self, runonceandblock: bool) -> JoinHandle<()>{
        std::thread::spawn(move || {
        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::with_template(
                "[{prefix:.cyan/blue}][{elapsed_precise}] Current    : {len:.cyan/blue}",
            )
            .unwrap()
        );

        bar.set_prefix("Stats");
        bar.set_message("Loading tests :");
        let lock = TrafficStarLogger::lock();
        let mp = match TrafficStarLogger::get_progress_bars(){
            Some(v) => v,
            None => {
                let mp = Arc::new(MultiProgress::new());
                TrafficStarLogger::set_progress_bars(mp.clone());
                mp
            },
        };
        let bar_current = Arc::new(Mutex::new(mp.add(bar)));
        let barlock = bar_current.lock().unwrap();
        barlock.enable_steady_tick(Duration::from_secs(1));
        barlock.force_draw();
        drop(barlock);

        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::with_template(
                "[{prefix:.cyan/blue}][{elapsed_precise}] Successful : {len:.green}",
            )
            .unwrap()
        );

        bar.set_prefix("Stats");
        bar.set_message("Loading tests :");
        bar.enable_steady_tick(Duration::from_secs(1));
        let bar_successful = Arc::new(Mutex::new(mp.add(bar)));

        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::with_template(
                "[{prefix:.cyan/blue}][{elapsed_precise}] Failed     : {len:.red}",
            )
            .unwrap()
        );

        bar.set_prefix("Stats");
        bar.set_message("Loading tests :");
        bar.enable_steady_tick(Duration::from_secs(1));

        let bar_failed = Arc::new(Mutex::new(mp.add(bar)));
        drop(lock);

        sinfo!("Server starting!");
        loop {
            sinfo!("Waiting for new client to handle!");
            let host = match self.listener.accept(){
                Ok(v) => {
                    sinfo!("Accepted connection from {}",v.1);
                    v
                },
                Err(err) => {
                    serror!("Failed accepting connection, error : {}",err);
                    continue;
                },
            };
            let rt = match get_multi_runtime(){
                Ok(v) => v,
                Err(err) => {
                    serror!("Could not create runtime for accepted connection, error : {}",err);
                    continue;
                },
            };
            let stream = host.0;
            match stream.set_nonblocking(true){
                Ok(_) => {},
                Err(err) => {
                    serror!("Couldn't make socket non_blocking, err : {}",err);
                    continue;
                },
            };

            let mut node_data = self.general_data.clone();
            let data_directory = self.data_directory.clone();
            let bar_current = bar_current.clone();
            let bar_successful = bar_successful.clone();
            let bar_failed = bar_failed.clone();
            
            let running_lock_copy = Arc::new(self.running_guard.new_shared());
            let thread = std::thread::spawn(move || {

                let barlock = bar_current.lock().unwrap();
                barlock.inc_length(1);
                drop(barlock);
                let rwlockcopy  = running_lock_copy.clone();
                match rt.block_on(async move {
                    let stream = tokio::net::TcpStream::from_std(stream)?;
                    let connection = create_connection_struct(stream).await?;
                    Self::handle_client(&mut node_data, ServerClientEntry { 
                        connection, 
                        their_interfaces: Vec::new(), 
                        data_directory,
                        rw_lock : rwlockcopy,
                        reported_route_info : None
                    }).await?;

                    Ok::<(),TrafficStarError>(())
                }){
                    Ok(_) => {
                        let barlock = bar_successful.lock().unwrap();
                        barlock.inc_length(1);
                        drop(barlock);
                        sinfo!("Client handled correctly!")
                    },
                    Err(err) => {
                        let barlock = bar_failed.lock().unwrap();
                        barlock.inc_length(1);
                        drop(barlock);
                        serror!("Received error handling client : {}",err)
                    },
                    
                };
                
                let barlock = bar_current.lock().unwrap();
                barlock.dec_length(1);
                drop(barlock);
                get_singleton_multi().block_on(running_lock_copy.uninterested())
            });
            if runonceandblock{
                thread.join().unwrap();
                break;
            }
        }
    })
    }


    #[allow(unused)]
    async fn run_sink(server: &mut ServerClientEntry, 
        node_data: &mut GeneralNodeData, 
        directory_output : Option<String>,
        filename_prefix : Option<String>) 
    -> Result<(),TrafficStarError>{
        let (mut listener,port) = create_tcp_listener(&node_data.route.ipv4)?;
        listener.set_nonblocking(true)?;
        let mut sink = SinkReceiver::new_listener(tokio::net::TcpListener::from_std(listener)?)?;
        let mut address_string: String =
        node_data.route.ipv4.to_string() + ":" + &port.port.to_string();
        let listen_address = address_string.clone();
        if node_data.route.ipv4_public.is_some() {
            address_string =
                node_data.route.ipv4_public.clone().unwrap() + ":" + &port.port.to_string();
        }
        let mut block_others = false;
        server.rw_lock.view().await;
        server.connection.send(HandshakeType::ReportDedicatedDataChannelSink).await?;
        server.connection.send(address_string).await?;
        

        let mut tcpdump = None;


        loop{
            let handshake = server.connection.read::<SinkHandShakeType>().await?;
            sdebug!("Read hanshake type : {}",handshake);
            match handshake{
                SinkHandShakeType::StartPcap => {
                    let filename: PathBuf = PathBuf::from(create_file_for_tcpdump_output(
                                &server.their_interfaces,
                                node_data,
                                server.data_directory.clone() ,
                                directory_output.clone(),
                                filename_prefix.clone()
                            )?);
                    sinfo!("Output file is : {:?}!",filename);

                    if tcpdump.is_none(){
                        if block_others{
                            sdebug!("Blocking others!");
                            server.rw_lock.block().await;
                        }else{
                            server.rw_lock.view().await;
                        }

                        tcpdump = Some(ASyncTCPDump::new(Some(&listen_address),
                        &node_data.route.interface_name, 
                        filename.clone()).await?);
                        server.connection.send(SinkHandShakeType::StartPcap).await?;
                    }else{
                        return Err("TCPDUMP was not none when expected!".into())
                    }
                },
                SinkHandShakeType::Finished => {
                    if let Some(tcpdump) = tcpdump.take(){
                        tcpdump.save_and_exit().await?;
                        sinfo!("Saved sinkreceiver pcap to file : {:?}!",tcpdump.output_file);

                        if let Some(connection_info) = &server.reported_route_info{
                            sinfo!("Saving info file!");
                            let mut path = tcpdump.output_file.to_path_buf();
                            if path.set_extension("info"){
                                sinfo!("Updated extension to .info!");
                                let mut file = tokio::fs::File::create(&path).await?;
                                file.write_all(connection_info.as_bytes()).await;
                                sinfo!("Saved file {:?}",path);
                            }else{
                                serror!("Couldn't update extension to .info!");
                            }
                        }

                        
                        sdebug!("Read all stdouts!");
                    }else{
                        return Err("Tried finishing when TCPDUMP was none!".into())
                    }
                },
                SinkHandShakeType::Stop => {
                    sinfo!("Stopping, sinkreceiver ran as expected!");
                    sink.kill().await;
                    sinfo!("Read {:.3} MiB, ({:.3} Mbps)",sink.get_transfered_mebibytes(), sink.get_speed_mbits());
                    
                    server.connection.send(SinkHandShakeType::Stop).await?;
                    return Ok(())
                },
                SinkHandShakeType::PreventSimultanousRecordings => {
                    block_others = server.connection.read::<bool>().await?;
                    sinfo!("Blocking : {}",block_others);
                },
            }
        }
    }

    #[allow(unused_assignments)]
    async fn run_proxy(server: &mut ServerClientEntry, 
        node_data: &mut GeneralNodeData, 
        directory_output : Option<String>,
        filename_prefix : Option<String>) -> Result<(), TrafficStarError> {
        let connection = &mut server.connection;
        let (listener,port) = create_tcp_listener(&node_data.route.ipv4)?;
        sdebug!("listener address : {}",listener.local_addr().unwrap());
        let mut address_string: String =
        node_data.route.ipv4.to_string() + ":" + &port.port.to_string();
        let listen_address = address_string.clone();
        if node_data.route.ipv4_public.is_some() {
            address_string =
                node_data.route.ipv4_public.clone().unwrap() + ":" + &port.port.to_string();
        }
        let mut proxy = HttpProxyServer::new(listener,None).await?;
   
        server.rw_lock.view().await;
        connection.send(HandshakeType::ReportDedicatedDataChannelProxy).await?;
        connection.send(address_string.clone()).await?;
        
        let mut tcpdump = None;
        let mut website_prefix : Option<String> = None;
        let mut block_others = false;
        loop{
            match connection.read::<ProxyHandShakeType>().await.map_err(|_| TrafficStarError::msg("Client communication lost, failed test?".into()))?{
                ProxyHandShakeType::StartPcap => {
                    let mut file_prefix = filename_prefix.clone().unwrap_or("".into());
                    if let Some(website_prefix) = &website_prefix{
                        if file_prefix.is_empty(){
                            file_prefix = format!("{{{}}}",website_prefix)
                        }else{
                            file_prefix = format!("{{{}}}",file_prefix);
                        }
                    }
                    let filename: PathBuf = PathBuf::from(create_file_for_tcpdump_output(
                                &server.their_interfaces,
                                node_data,
                                server.data_directory.clone() ,
                                directory_output.clone(),
                                match file_prefix.is_empty(){
                                    true => None,
                                    false => Some(file_prefix),
                                }
                            )?);
                    sinfo!("Output file is : {:?}!",filename);
                    
                    if tcpdump.is_none(){
                        if block_others{
                            server.rw_lock.block().await;
                        }else{
                            server.rw_lock.view().await;
                        }

                        tcpdump = Some(ASyncTCPDump::new(Some(&listen_address),
                        &node_data.route.interface_name, 
                        filename.clone()).await?);
                        connection.send(ProxyHandShakeType::StartPcap).await?;
                    }else{
                        return Err("TCPDUMP was not none when expected!".into())
                    }
                },
                ProxyHandShakeType::Finished => {
                    if let Some(tcpdump) = tcpdump.take(){
                        proxy.restart().await?;
                        tokio::time::sleep(Duration::from_secs(1)).await;

                        server.rw_lock.uninterested().await;

                        if let Err(err) = tcpdump.save_and_exit().await{
                            return Err(err);
                        }else{
                            sinfo!("Saved test to pcap file {:?}",tcpdump.output_file.as_path());

                            if let Some(connection_info) = &server.reported_route_info{
                                sinfo!("Saving info file!");
                                let mut path = tcpdump.output_file.to_path_buf();
                                if path.set_extension("info"){
                                    sinfo!("Updated extension to .info!");
                                    let mut file = tokio::fs::File::create(&path).await?;
                                    file.write_all(connection_info.as_bytes()).await?;
                                    sinfo!("Saved file {:?}",path);
                                }else{
                                    serror!("Couldn't update extension to .info!");
                                }
                            }

                            connection.send(ProxyHandShakeType::Finished).await?;
                            continue;
                        }
                    }else{
                            return Err("TCPDUMP not started when finished sent!".into())
                    }
                }
                ProxyHandShakeType::NextTest => {
                    tcpdump = None;
                    server.rw_lock.uninterested().await;
                    server.rw_lock.block().await;
                },
                ProxyHandShakeType::Stop => {
                    proxy.kill().await?;
                    drop(port);
                    return Ok(())
                }
                ProxyHandShakeType::ReportWebsite => {
                    if let Ok(url) = Url::parse(&connection.read::<String>().await?)
                    && let Some(host) = url.domain(){
                        website_prefix = Some(host.to_string());
                    }
                },
                ProxyHandShakeType::PreventSimultanousRecordings => {
                    block_others = connection.read::<bool>().await?;
                    sinfo!("Block concurrent recordings : {}",block_others);
                },
            }
        }
        
    }

    async fn handle_client(node_data: &mut GeneralNodeData, mut server: ServerClientEntry) -> Result<(), TrafficStarError>{
        sinfo!("Handling client!");
        sinfo!("Server sending traversel types!");
        if let Err(v) = crate::trafficstar_nodes::send_connection_types(node_data, &mut server.connection).await{
            serror!("Could not send all link types, error {}",v);
        }
        sinfo!(
            "Handling connection for {}",
            server.connection.peer_addr
        );

        sinfo!("Server sent our traversel types!");
        sinfo!("Reading handshake packets from client!");
        let mut directory_output: Option<String> = None;
        let mut filename_prefix : Option<String> = None;
        loop {
            sinfo!("Waiting for handshake packet!");
            let res = server.connection.read::<HandshakeType>().await?;
            sinfo!("Read HandShakeType : {}",res);
            match res {
                HandshakeType::ReportTestDirectoryPrefix => {
                    directory_output = Some(server.connection.read::<String>().await?);
                    sinfo!(
                        "Received ReportTestDirectoryPrefix, output directory set to : {}",
                        directory_output.clone().unwrap()
                    )
                },
                HandshakeType::ReportRouteInfo => {
                    server.reported_route_info = server.connection.read::<Option<String>>().await?;
                    sinfo!("Received reported server info : {:?}",server.reported_route_info);
                }
                HandshakeType::SetNickParts => {
                    let nick_part = server.connection.read::<Vec<String>>().await?;
                    TrafficStarLogger::remove_nick_thread();
                    for nick in nick_part{
                        TrafficStarLogger::add_nick_thread(nick.clone());
                    }
                    TrafficStarLogger::set_threadhook_nick(TrafficStarLogger::get_nick_thread());
                }

                HandshakeType::ReportTestFileNamePrefix => {
                    filename_prefix = Some(server.connection.read::<String>().await?);
                    sinfo!(
                        "Received ReportTestFileNamePrefix, set file-name prefix to: {}",
                        filename_prefix.clone().unwrap()
                    )
                }

                HandshakeType::ReportConnectionTypes => {
                    let traverseltype = server.connection.read::<Vec<ConnectionType>>().await?;
                    server.their_interfaces.clear();
                    for traverseltype in traverseltype.clone(){
                        server.their_interfaces.push(traverseltype);
                    }
                    sinfo!(
                            "Server set their traversal types to : {:?}",
                            traverseltype
                    );
                }

                HandshakeType::RunSink => {
                    sinfo!(
                        "Running RunSink!"
                    );
                    match Self::run_sink(&mut server, node_data, directory_output.clone(), filename_prefix.clone()).await{
                        Ok(_) => {
                            sinfo!("Sink ran correctly.")
                        },
                        Err(err) => {
                            return Err(err)
                        },
                    }
                    sinfo!("Server child handler stopping!");
                    return Ok(())
                }

                HandshakeType::RunProxy => {
                    sinfo!("Running proxy!");
                    
                    match Self::run_proxy(&mut server, node_data,directory_output.clone(),filename_prefix.clone()).await{
                            Ok(_) => {
                                sinfo!("Ran proxy correctly!");
                                return Ok(())
                            },
                            Err(v) => {
                                return Err(v);
                            },
                        }
                }

                v => {
                    return Err(format!("Received unexpected HandShakeType {}",v).into());
                }
            }
        
        }
    }
}

struct PortReserv{
    port : u16
}
///Creates a tcplistener!
fn create_tcp_listener(ipv4_addr : &str) -> Result<(TcpListener,PortReserv), TrafficStarError> {
    let mut ports = USED_PORTS.lock().expect("Couldn't get lock!");
    const LAST_PORT_TO_CHECK: u16 = 65535;
    const FIRST_PORT_TO_CHECK: u16 = 5201;
    for port in FIRST_PORT_TO_CHECK..LAST_PORT_TO_CHECK {
        if !ports.contains(&port) && let Ok(l) = TcpListener::bind((ipv4_addr, port))
        {
            ports.push(port);
            return Ok((l,PortReserv{
                port
            }));
        }
    }

    Err(TrafficStarError::id_msg("NoFreePort".into(), "Couldn't find a free port!".into()))
}

impl Drop for PortReserv {
    fn drop(&mut self) {
        let mut ports = USED_PORTS.lock().expect("Couldn't get lock!");
        if let Some(position) = ports.iter().position(|x|*x==self.port){
            ports.remove(position);
        }
    }
}
