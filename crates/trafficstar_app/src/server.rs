use std::{net::{SocketAddr}, path::PathBuf};

use clap::ArgMatches;
use socket2::{Domain, Protocol, Socket, Type};
use trafficstar_connections::trafficstar_data_route::DataRoute;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{serror, sinfo, swarn};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_nodes::trafficstar_node_server::ServerNode;
use trafficstar_utilities::run_command;

use crate::configurations::Configuration;

#[derive(StructLoggerName)]
pub struct Server{
    interface : DataRoute,
    storage : PathBuf,
    port : u16
}

impl Server{
    pub fn new(matches : ArgMatches) -> Result<Self, TrafficStarError>{
        let config = Configuration::try_from(matches)?;
        let mut interface : Option<DataRoute> = None;
        let interfaces = config.interfaces.as_path();
        if !interfaces.exists() || !interfaces.is_dir(){
            return Err(TrafficStarError::msg("Interfaces directory specified doesn't exist or is not a directory!".into()))
        }
        for file in std::fs::read_dir(interfaces)?{
            let file = match file{
                Ok(v) => v,
                Err(err) => {
                    serror!("Error reading file : {}!",err);
                    continue;
                },
            };

            if file.path().is_file(){
                match DataRoute::from_json(&file.path()){
                    Ok(v) => {
                        sinfo!("Append interface file {:?}, entry : {}!",file.path(),v);
                        if v.interface_name == config.addr.0 || v.ipv4 == config.addr.0{
                            interface = Some(v);
                            break;
                        }
                    },
                    Err(err) => {
                        swarn!("Failed reading interface file {:?}, error : {}",file.path(), err);
                    },
                };
            }else{  
                swarn!("Ignoring direntry  {:?} as it's not a file!",file.path());
            }
        }

        if interface.is_none(){
            return Err(format!("Did not find an interface with the name or address {}!",config.addr.0).into())
        }
        let interface = interface.unwrap();
        if !config.storage.is_dir() {
            return Err("Specified storage directory is not a existing directory!".into())
        }

        Ok(Self { interface, storage: config.storage, port : config.addr.1 })
    }


    pub fn run(&self) -> Result<(),TrafficStarError>{
        let address = format!("0.0.0.0:{}",self.port);
        sinfo!("Binding to interface {} on port {}, {}",self.interface.interface_name,self.port,address);
        sinfo!("Broadcaste ip will be : {}", match &self.interface.ipv4_public{
            Some(v) => v,
            None => {
                &self.interface.ipv4
            },
        });
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.bind_device(Some(self.interface.interface_name.as_bytes()))?;
        socket.bind(&address.parse::<SocketAddr>()?.into())?;
        socket.listen(128)?;
        let listener = socket.into();
        
        swarn!("Disabling tso and etc!\n***WARNING***\nIF RUNNING IN A CONTAINER, DISABLE TCP OFFLOADING AND ETC ON THE USED PHYSICAL NIC!!!");
        run_command("ethtool", vec!["-K",&self.interface.interface_name,
        "gro","off",
        "lro","off",
        "tso","off",
        "tx-sctp-segmentation","off"])?;


        let node = ServerNode::init_server(listener, self.interface.clone(), self.storage.clone());


        if let Err(err) = node.run_server(false).join(){
            Err(format!("{:?}",err).into())
        }else{
            Ok(())
        }

    }

}