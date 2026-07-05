use core::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{PathBuf};
use std::sync::Arc;


use chrono::Local;
use log::{error};
use trafficstar_connections::trafficstar_data_traversel_types::ConnectionType;
use trafficstar_connections::trafficstar_connection::{Connection};
use trafficstar_connections::trafficstar_enums::HandshakeType;
use trafficstar_connections::{
    trafficstar_data_route::DataRoute as DataRoute
};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_errors::trafficstar_error_traits::TrafficStarEnumErrorTrait;


#[derive(Clone)]
pub struct GeneralNodeData {
    pub route: DataRoute,
}

pub async fn send_connection_types(node_data: &mut GeneralNodeData, connection: &mut Connection) -> Result<(),TrafficStarError>{
    connection.send(HandshakeType::ReportConnectionTypes).await?;
    connection.send(node_data.route.data_traversals.clone()).await?;
    Ok(())
}

#[derive(Debug,Clone, strum_macros::EnumMessage)]
pub enum NodeErrorKind{
    #[strum(message = "Failed to receive start process.")]
    FailedToStart,
    #[strum(message = "Process had an error")]
    ProccessError,
    #[strum(message = "Communication error")]
    CommunicationError,
    #[strum(message = "TrafficStarError")]
    TrafficStarErrorKind,
}
#[derive(Debug, Clone)]
pub struct NodeError {
    pub kind : NodeErrorKind,
    pub key : String,
    pub info : Option<String>,
    
}

impl NodeError{
    pub fn startfailure(field : &str, info : Option<String>) -> Self{
        NodeError { 
            kind: NodeErrorKind::FailedToStart, 
            key: field.to_string(),
            info,
        }
    }

    pub fn proccesserror(field : &str, info : Option<String>) -> Self{
        NodeError { 
            kind: NodeErrorKind::ProccessError, 
            key: field.to_string(), 
            info,
        }
    }
    pub fn communicationerrror(field : &str, info : Option<String>) -> Self{
        NodeError { 
            kind: NodeErrorKind::CommunicationError, 
            key: field.to_string(), 
            info,
        }
    }

    pub fn trafficstar_error(trafficstar_error : TrafficStarError) -> Self{
        NodeError { kind: NodeErrorKind::TrafficStarErrorKind, key: format!("{}",trafficstar_error), info : None }
    }
}

impl TrafficStarEnumErrorTrait for NodeError{
    fn enum_name(&self) -> &str {
        self.kind.enum_name()
    }

    fn enum_variant(&self) -> &str {
        self.kind.enum_variant()
    }

    fn enum_message(&self) -> Option<String> {
        Some(format!("{}",self))
    }
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind{
            NodeErrorKind::FailedToStart => {
                match self.info.clone() {
                    Some(v) =>{
                        write!(f,"Failed to start {{{}}}, message {{{}}}!",self.key,v)
                    }
                    None => {
                        write!(f,"Failed to start {{{}}}!",self.key)
                    }
                }
            },
            NodeErrorKind::ProccessError => {
                 match self.info.clone() {
                    Some(v) =>{
                        write!(f,"Proccess error from {{{}}}, message {{{}}}!",self.key,v)
                    }
                    None => {
                        write!(f,"Proccess error from {{{}}}!",self.key)
                    }
                }
            },
            NodeErrorKind::CommunicationError => {
                 match self.info.clone() {
                    Some(v) =>{
                        write!(f,"Communication error for {{{}}}, message {{{}}}!",self.key,v)
                    }
                    None => {
                        write!(f,"Communication error for {{{}}}!",self.key)
                    }
                }
            },
            NodeErrorKind::TrafficStarErrorKind =>  {
                write!(f,"{}",self.key)
            }
        }
    }
}

impl From<NodeError> for TrafficStarError{
    fn from(value: NodeError) -> Self {
        TrafficStarError::enums(Arc::new(value))
    }
}

impl From<TrafficStarError> for NodeError{
    fn from(value: TrafficStarError) -> Self {
        Self::trafficstar_error(value)
    }
}

fn create_file_name(their_interfaces: &Vec<ConnectionType>,
    node_data: &mut GeneralNodeData,
    directory : PathBuf,
    sub_directory: Option<String>,
    filename_prefix : Option<String>,) -> Result<PathBuf, TrafficStarError>
{    
    let mut file_path = directory;
    if let Some(dir) = sub_directory {
        file_path = file_path.join(dir);
    }
    let mut first = true;
    let mut file_name = "".to_string();
    for i in their_interfaces {
        let mut to_append = i.to_string();

        if !first {
            to_append = "-".to_string().to_owned() + &to_append;
        }
        file_name = file_name + &to_append;
        first = false;
    }
    if !file_name.is_empty(){
        file_path = file_path.join(file_name);
    }

    file_name = "".to_string();
    let mut first = true;

    for i in node_data.route.data_traversals.iter() {
        let mut to_append = i.to_string();

        if !first {
            to_append = "-".to_owned() + &to_append;
        }
        file_name += &mut to_append;
        first = false;
    }
    if !file_name.is_empty(){
        file_path = file_path.join(file_name);
    }
    let res = fs::create_dir_all(file_path.clone());
        if res.is_ok() {
            //info!("Created directory {} without errors!", directory);
        }
        if let Err(err) = res {
            if err.kind() != ErrorKind::AlreadyExists {
                error!(
                    "Could not create directory {}, got error : {}",
                    file_path.clone().to_str().unwrap(),
                    err.kind()
                );
                return Err(err.into())
            } else {
                //info!("Directory {} already exists", directory);
            }
        }
    file_name = "".to_string();
    if let Some(filename_prefix) = filename_prefix{
        file_name+=&filename_prefix;
        file_name+="-";
    }
    file_name += &Local::now().format("%Y-%m-%d-%H:%M:%S").to_string();
    file_name += "-";
    file_name += &node_data.route.interface_name;
    
    Ok(file_path.join(file_name))
}

pub fn create_file_for_tcpdump_output(
    their_interfaces: &Vec<ConnectionType>,
    node_data: &mut GeneralNodeData,
    directory : PathBuf,
    sub_directory: Option<String>,
    filename_prefix : Option<String>,
) -> Result<String,TrafficStarError> {
    let file_path = match create_file_name(their_interfaces, node_data, directory.clone(), sub_directory.clone(), filename_prefix.clone())?.to_str()
    {
        Some(v) => v.to_string(),
        None => return Err("Could not create a valid file path!".into()),
    };
    
    let mut loops = 0;
    loop{
        
    
        let save_path= PathBuf::from(format!("{}-{}.pcap",file_path,loops));
        
        match std::fs::File::create_new(save_path.as_path())
        {
            Ok(v) => {
                drop(v)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AlreadyExists{
                    loops += 1;
                    continue;
                }
                return Err(format!("{} : {}",save_path.to_str().unwrap_or("?"),err).into())
            },
        };
        match save_path.to_str(){
            Some(v) => return Ok(v.to_string()),
            None => return Err(TrafficStarError::msg("Couldn't generate file path!".into())),
        }
    }

}