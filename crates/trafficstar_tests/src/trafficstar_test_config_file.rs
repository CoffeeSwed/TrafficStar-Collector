use std::{fs::File, io::Read, path::Path};

use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_processes::{trafficstar_mullvad_browser::MullvadBrowserSettings};
use trafficstar_utilities::sink::settings::SinkSenderSettings;

#[derive(Clone, Default,serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TrafficStarTestConfigFile {
    pub sinkparams : Option<SinkSenderSettings>,
    pub mullvadbrowserparams : Option<MullvadBrowserSettings>,
    pub test_parameters : TrafficStarTestParameters,
    pub do_for_tag : Vec<String>,
}

#[derive(Clone, Default,serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TrafficStarTestParameters{
    pub name : Option<String>,
    pub samples : usize,
    pub parallel: usize,
}

impl std::fmt::Display for TrafficStarTestParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TrafficStarTestParameters{{name = {}, samples = {}, do_for_tag = {:?}}}",
               self.name.clone().unwrap_or("None".to_string()),
               self.samples,
               self.parallel,
        )
    }
}

impl std::fmt::Display for TrafficStarTestConfigFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"TrafficStarTestConfigFile{{test_parameters = {}, iperf3params = {}, mullvadbrowserparams = {}, do_for_tag = {:?}}}",
               self.test_parameters.clone(),
               match self.sinkparams.clone(){
                   Some(v) => {
                       format!("{}",v)
                   }
                   None => {
                       "None".to_string()
                   }
               },
               match self.mullvadbrowserparams.clone(){
                   Some(v) => {
                       format!("{}",v)
                   }
                   None => {
                       "None".to_string()
                   }
               },
               self.do_for_tag.clone(),
        )
    }
}

impl TrafficStarTestConfigFile{
     pub fn from_json(path : &Path) -> Result<Self,TrafficStarError>{
        let mut file = File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        let string = String::from_utf8(buffer).map_err(|e| format!("Could not read file content as a string, error : {}",e))?;
        let serializer = serde_json::from_str(&string).map_err(|e| format!("Could not read data_route, error : {}",e))?;
        Ok(serializer)
    }

    pub fn from_dirs_json(dir : &Path) -> Result<Vec<Result<Self, TrafficStarError>>,std::io::Error>{
        if dir.is_dir(){
            let entries = dir.to_path_buf().read_dir()?;
            let mut results : Vec<Result<Self,TrafficStarError>> = Vec::new();
            for entry in entries{
                let entry = entry?.path();
                if entry.is_file(){
                     results.push(Self::from_json(&entry));
                }
            }
            return Ok(results)
        }
        Err(std::io::Error::new(std::io::ErrorKind::NotADirectory, "Given path now a file!"))
    }
       
}