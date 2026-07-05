use std::{fs::File, io::{Read}, path::Path};

use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::trafficstar_data_traversel_types::ConnectionType;

#[derive(Serialize,Deserialize)]
pub struct DataRoute {
    pub interface_name: String,
    pub ipv4: String,
    pub ipv4_public: Option<String>,
    pub tag: String,
    pub fwmark : Option<u32>,
    pub data_traversals: Vec<ConnectionType>,
    pub info : Option<String>
}

impl DataRoute{
    pub fn from_json(path : &Path) -> Result<Self,TrafficStarError>{
        let mut file = File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        let string = String::from_utf8(buffer).map_err(|e| format!("Could not read file content as a string, error : {}",e))?;
        let serializer = serde_json::from_str(&string).map_err(|e| format!("Could not read data_route, error : {}",e))?;
        Ok(serializer)
    }
}

pub trait Route{
    fn get_interface_name() -> String;
    
    ///IPV4 address for binding, or an interface name.
    fn get_ipv4_eq() -> String;

    ///Address usually sent from server to client informaing where to connect to next.
    fn get_ipv4_public() -> Option<String>;

    ///Tag identifying the type of the connection or something similar.
    fn get_tag() -> String;

    ///States the fwmark to set other routes to for forwarding through this route.
    fn get_fwmark() -> Option<String>;

}

impl std::fmt::Display for DataRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result: String = String::new();
        result = result + "{name : " + &self.interface_name + ", data_traversals : [";
        for i in 0..self.data_traversals.len() {
            if i != 0 {
                result += ", ";
            }
            result += &self.data_traversals[i].to_string();
        }
        result = result
            + "], tag : "
            + &self.tag.clone()
            +", fwmark : ";
        
        if let Some(fwmark) = self.fwmark{
            result += &fwmark.to_string()
        }
        else{
            result += "None"
        }

        result = result
            + ", ipv4 : "
            + &self.ipv4.clone()
            + ", ipv4_public : ";
        if self.ipv4_public.is_none() {
            result += "None";
        } else {
            result += &self.ipv4_public.clone().unwrap();
        }

        result += "}";
        write!(f, "{}", result)
    }
}

impl PartialEq for DataRoute{
    fn eq(&self, other: &Self) -> bool {
        if self.interface_name == other.interface_name && self.ipv4 == other.ipv4 && self.ipv4_public == other.ipv4_public && self.tag == other.tag && self.fwmark == other.fwmark
            && other.data_traversals.len() == self.data_traversals.len(){
                for i in 0..self.data_traversals.len(){
                    if self.data_traversals[i] != other.data_traversals[i]{
                        return false;
                    }
                }
                return true
        }
        false
    }
}

impl Clone for DataRoute {
    fn clone(&self) -> DataRoute {
        let mut traversels: Vec<ConnectionType> = Vec::new();
        for i in 0..self.data_traversals.len() {
            traversels.push(self.data_traversals[i]);
        }
        DataRoute {
            interface_name: self.interface_name.clone(),
            ipv4: self.ipv4.clone(),
            ipv4_public: self.ipv4_public.clone(),
            data_traversals: traversels,
            tag: self.tag.clone(),
            fwmark : self.fwmark,
            info : self.info.clone()
        }
    }
}



pub fn create_route(
    the_interface_name: String,
    the_ipv4: String,
    the_interface_travels: Vec<ConnectionType>
) -> DataRoute {
    DataRoute {
        interface_name: the_interface_name,
        ipv4: the_ipv4,
        ipv4_public: None,
        tag: "".to_string(),
        data_traversals: the_interface_travels,
        fwmark : None,
        info : None,
    }
}

pub fn create_route_nat(
    the_interface_name: String,
    the_ipv4: String,
    public_ipv4 : String,
    the_interface_travels: Vec<ConnectionType>,
) -> DataRoute {
    DataRoute {
        interface_name: the_interface_name,
        ipv4: the_ipv4,
        ipv4_public: Some(public_ipv4),
        tag: "".to_string(),
        data_traversals: the_interface_travels,
        fwmark : None,
        info : None
    }
}

impl DataRoute {
    pub fn from_default_interface() -> Result<DataRoute, std::io::Error> {
        Ok(DataRoute{
            interface_name : trafficstar_utilities::get_public_interface_name()?,
            fwmark : Some(16),
            tag : ConnectionType::Ethernet.to_string(),
            ipv4 : trafficstar_utilities::get_public_interface_ipv4_str()?,
            ipv4_public : Some(trafficstar_utilities::fetch_public_ip()),
            data_traversals : vec![ConnectionType::Internal,ConnectionType::Ethernet],
            info : None
        })
    }
}