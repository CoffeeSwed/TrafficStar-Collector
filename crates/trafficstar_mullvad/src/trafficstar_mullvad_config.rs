use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MullvadRelayConfig {
    pub entry: String,
    pub exit: Option<String>,
    pub tag: String,
    pub do_for_tags: Vec<String>,
}

impl MullvadRelayConfig{
     pub fn from_json(path : &Path) -> Result<Self,TrafficStarError>{
        let mut file = File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        let string = String::from_utf8(buffer).map_err(|e| format!("Could not read file content as a string, error : {}",e))?;
        let serializer = serde_json::from_str(&string).map_err(|e| format!("Could not read data_route, error : {}",e))?;
        Ok(serializer)
    }
}

impl Default for MullvadRelayConfig {
    fn default() -> Self {
        Self {
            entry: "".to_string(),
            exit: None,
            tag: "Default".to_string(),
            do_for_tags: Vec::new(),
        }
    }

    
}

impl std::fmt::Display for MullvadRelayConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result: String = String::new();
        result += "MullvadRelayConfig{entry : ";
        result += &self.entry;
        if let Some(exit) = self.exit.clone() {
            result += ", exit : ";
            result += &exit;
        } else {
            result += ", exit : None"
        }
        result += &format!(", tag : {}",self.tag);
        result += ", do_for_tags : [";
        for (index, tag) in self.do_for_tags.iter().enumerate() {
            if index == 0 {
                result += tag;
            } else {
                result += ", ";
                result += tag;
            }
        }
        result += "]";

        let _ = write!(f, "{{{}}}", result);
        Ok(())
    }
}
