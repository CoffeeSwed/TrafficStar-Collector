use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger_macro::StructLoggerName;

#[derive(StructLoggerName, Deserialize, Serialize, Default, Clone)]
pub struct TorVPNConfig{
    pub do_for_tags : Vec<String>,
    pub tag : String
}

impl TorVPNConfig{
    pub fn from_json(path : &Path) -> Result<Self,TrafficStarError>{
        let mut file = File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        let string = String::from_utf8(buffer).map_err(|e| format!("Could not read file content as a string, error : {}",e))?;
        let serializer = serde_json::from_str(&string).map_err(|e| format!("Could not read data_route, error : {}",e))?;
        Ok(serializer)
    }
}

impl std::fmt::Display for TorVPNConfig{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"TorVPNConfig{{ do_for_tags : {:?}, tag : {}}}",&self.do_for_tags,&self.tag)
    }
}