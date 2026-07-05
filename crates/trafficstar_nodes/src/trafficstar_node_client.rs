use std::{path::PathBuf, str::FromStr, time::Duration};

use crate::trafficstar_nodes::{self, GeneralNodeData};
use tokio::{net::TcpStream, time::Instant};
use trafficstar_connections::{ trafficstar_data_traversel_types::ConnectionType, trafficstar_connection::{self as Connections, Connection}, trafficstar_data_route::DataRoute as DataRoute, trafficstar_enums::{HandshakeType, ProxyHandShakeType, SinkHandShakeType}
};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_interface::LinkType;
use trafficstar_logger::{sdebug, serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_processes::{trafficstar_mullvad_browser::{MullvadBrowserProccess, MullvadBrowserRun}};
use trafficstar_utilities::{sink::{sender::SinkSender, settings::SinkSenderSettings}, trafficstar_networking::interface::TrafficStarInterfaceName};



#[derive(StructLoggerName)]
pub struct ClientNode {
    pub general_data: GeneralNodeData,
    pub connection: Connection,
    pub their_connection_types: Vec<ConnectionType>,
    pub data_directory : PathBuf,
}
impl ClientNode{
    pub async fn init_client(connection: TcpStream, the_route: DataRoute, data_directory : PathBuf,
        nick_parts : Option<Vec<String>>
    ) -> Result<Self, TrafficStarError> {
        let mut res = ClientNode {
            general_data: GeneralNodeData { route: the_route },
            connection: Connections::create_connection_struct(connection).await?,
            their_connection_types: Vec::new(),
            data_directory
        };
        res.initial_handshake(nick_parts).await?;

        Ok(res)
    }

    pub async fn initial_handshake(&mut self, nick_parts : Option<Vec<String>>) -> Result<(), TrafficStarError>{
        trafficstar_nodes::send_connection_types(&mut self.general_data, &mut self.connection).await?;
        if let Some(nick) = nick_parts{
            self.connection.send(HandshakeType::SetNickParts).await?;
            self.connection.send(nick).await?;
        }
        let message = self.connection.read::<HandshakeType>().await?;
        if message == HandshakeType::ReportConnectionTypes{
            let connection_types = self.connection.read::<Vec<ConnectionType>>().await?;
            self.their_connection_types.clear();
            for traverseltype in connection_types.clone(){
                self.their_connection_types.push(traverseltype);
            }
            sinfo!(
                    "Client set their connection types to : {:?}",
                    connection_types
            );
        }else{
            return Err(format!("Unexpected handshake type received on initial setup, received {} but expected {}!",message,HandshakeType::ReportConnectionTypes).into())
        }
        self.connection.send(HandshakeType::ReportRouteInfo).await?;
        self.connection.send(self.general_data.route.info.clone()).await?;
        
        Ok(())

    }

    
    #[allow(unused)]
    pub async fn run_client_sink(&mut self, 
        output_directory: String,
        sink_sender_settings : &SinkSenderSettings,
        filename_prefix : Option<String>,
        block_concurrent_recordings : bool,
    ) -> Result<(),TrafficStarError>{
         let interface_name = TrafficStarInterfaceName::from_str(&self.general_data.route.interface_name)?;
        sdebug!("Sending TestDirectoryPrefix!");
        //Directory prefix
        self.connection.send(HandshakeType::ReportTestDirectoryPrefix).await?;
        self.connection.send(output_directory.clone()).await?;
        //Filename prefix
        if let Some(filename_prefix) = filename_prefix.clone(){
            sdebug!("Sending filename prefix!");

            self.connection.send(HandshakeType::ReportTestFileNamePrefix).await?;
            self.connection.send(filename_prefix).await?;
        }
        
        sdebug!("Sending we want to run Sink!");
        self.connection.send(HandshakeType::RunSink).await?;
        
        match self.connection.read::<HandshakeType>().await?{

            HandshakeType::ReportDedicatedDataChannelSink => {
                sdebug!("ReportDedicatedDataChannelSink!");
                let address = self.connection.read::<String>().await?;
                let address = match address.parse(){
                    Ok(v) => v,
                    Err(err) => return Err(format!("Failed to parse received address {}, error : {}",address,err).into()),
                };
                self.connection.send(SinkHandShakeType::PreventSimultanousRecordings).await?;
                self.connection.send(block_concurrent_recordings).await?;
                self.connection.send(SinkHandShakeType::StartPcap).await?;
                match self.connection.read::<SinkHandShakeType>().await?{
                    SinkHandShakeType::StartPcap => {
                        sdebug!("Starting SinkSender!");
                        let timeout = sink_sender_settings.time + 10;
                        let stream = tokio::net::TcpSocket::new_v4()?;
                        stream.bind_device(Some(self.general_data.route.interface_name.as_bytes()))?;
                        sdebug!("Stream binded to {}",self.general_data.route.interface_name);
                        let stream = stream.connect(address).await?;
                        sdebug!("Connected");
                        let starttime = Instant::now();
                        let sinksender = SinkSender::new(stream)?;
                        sdebug!("Sink sender started, sleeping for {}s",sink_sender_settings.time);
                        tokio::time::sleep(Duration::from_secs(sink_sender_settings.time as u64)).await;
                        let runtime = match starttime.elapsed().as_secs(){
                            0 => {
                                1
                            }
                            v => {
                                v
                            }
                        } as u128;
                        let sentbytes = sinksender.kill().await?;
                        let mbps = (sentbytes as f64)*8.0;
                        let mbps = (mbps) / ((1_000_000.0)*runtime as f64);                        
                        sdebug!("Sent {:.3} MiB, ({:.3} Mbps)", ((sentbytes as f64)/(1024.0*1024.0)),mbps);
 

                    },
                    v => {
                        return Err(format!("Expected {}, received {}!",SinkHandShakeType::StartPcap,v).into())
                    }
                }
                


                  
                self.connection.send(SinkHandShakeType::Finished).await?;
                self.connection.send(SinkHandShakeType::Stop).await?;
                let _ = self.connection.read::<SinkHandShakeType>().await;

                
                Ok(())
            },

            v => {
                Err(format!("Didn't receive {} as expected, got instead : {}", HandshakeType::ReportDedicatedDataChannelSink,v).into())
            }
        }
    }
    

    pub async fn run_client_browser(
         &mut self,
        directory_prefix: String,
        browser_run : &MullvadBrowserRun,
        filename_prefix : Option<String>,
        layer_3_forwarding : Option<LinkType>,
        block_concurrent_on_record : bool,
    ) -> Result<(), TrafficStarError>{
        let interface_name = TrafficStarInterfaceName::from_str(&self.general_data.route.interface_name)?;
        sdebug!("Sending TestDirectoryPrefix!");
        //Directory prefix
        self.connection.send(HandshakeType::ReportTestDirectoryPrefix).await?;
        self.connection.send(directory_prefix.clone()).await?;
        sdebug!("Sending filename prefix!");
        //Filename prefix
        if let Some(filename_prefix) = filename_prefix.clone(){
            self.connection.send(HandshakeType::ReportTestFileNamePrefix).await?;
            self.connection.send(filename_prefix).await?;
        }
        
        sdebug!("Sending we want to run proxy!");
        self.connection.send(HandshakeType::RunProxy).await?;
        
        match self.connection.read::<HandshakeType>().await?{

            HandshakeType::ReportDedicatedDataChannelProxy => {
                sdebug!("ReportDedicatedDataChannelProxy!");
                let address = self.connection.read::<String>().await?;
                self.connection.send(ProxyHandShakeType::PreventSimultanousRecordings).await?;
                self.connection.send(block_concurrent_on_record).await?;

                    self.connection.send(ProxyHandShakeType::ReportWebsite).await?;
                    self.connection.send(browser_run.website.to_string()).await?;

                    let browser_copy = browser_run.clone();
                    let mut process = MullvadBrowserProccess::new(
                        &browser_copy.test_script,
                        interface_name, 
                        layer_3_forwarding.clone(),
                    browser_run.max_progress_time).await?;
                        
                    process.write("Confirmed").await?;
                    process.write(&browser_run.website).await?;
                    process.write(&address).await?;
                    
                    loop{
                        let line = process.read_line().await?;

                        if line.to_ascii_lowercase().starts_with("$input : signal_ready"){
                            self.connection.send(ProxyHandShakeType::StartPcap).await?;
                            match self.connection.read::<ProxyHandShakeType>().await?{
                                ProxyHandShakeType::StartPcap => {

                                },
                                v => {
                                    return Err(format!("Expected StartPcap, got instead {}",v).into())
                                },
                            };
                            
                            process.write("Confirmed").await?;
                        }
                        
                        if line.to_ascii_lowercase().starts_with("$result"){
                            process.wait().await?;
                            self.connection.send(ProxyHandShakeType::Finished).await?;
                            if self.connection.read::<ProxyHandShakeType>().await? != ProxyHandShakeType::Finished{
                                return Err("Received wrong signal after finishing a website run from the server!".into())
                            }
                            break;
                        }

                        if line.to_ascii_lowercase().starts_with("$error"){
                            serror!("Received error : {}",line);
                            process.wait().await?;
                            break;
                        }
                        sinfo!("{}",line);

                    }

                  
                    self.connection.send(ProxyHandShakeType::Stop).await?;
                    
                
                Ok(())
            },

            v => {
                Err(format!("Didn't receive {} as expected, got instead : {}", HandshakeType::ReportDedicatedDataChannelProxy,v).into())
            }
        }
    }
}