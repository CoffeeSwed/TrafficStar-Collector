
use std::{path::PathBuf, str::FromStr};

use async_trait::async_trait;
use trafficstar_connections::{ trafficstar_data_traversel_types::ConnectionType, trafficstar_data_route::DataRoute};
use trafficstar_errors::{traffic_star_error::TrafficStarError};
use trafficstar_logger::{sdebug, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_processes::trafficstar_mullvad_browser::MullvadBrowserRun;
use trafficstar_tor::{TorInterfaceConfig, tor_process::{TorProcess, tor_vpn_config::TorVPNConfig}};
use trafficstar_utilities::trafficstar_networking::interface::TrafficStarInterfaceName;
use uuid::Uuid;


use crate::{trafficstar_test::{TrafficStarTestHandlersTraits, TrafficStarTestSession}, trafficstar_test_config_file::TrafficStarTestConfigFile};
#[derive(StructLoggerName)]
pub struct TorTestHandler{
    combinations_sink : Vec<DataRoute>,
    combinations_mullvad : Vec<(DataRoute,MullvadBrowserRun)>,

    pub config : TrafficStarTestConfigFile,
    pub vpn_config : Vec<TorVPNConfig>,
    uuid : Uuid
}

impl TorTestHandler{
    pub fn new(config : TrafficStarTestConfigFile, routes : Vec<DataRoute>, vpn_config : Vec<TorVPNConfig>) -> Self{
        let mut res = Self{
            combinations_sink : Vec::new(),
            combinations_mullvad : Vec::new(),
            config : config.clone(),
            vpn_config : vpn_config.clone(),
            uuid : Uuid::new_v4()
        };
        for vpn_config in &vpn_config{
            if config.do_for_tag.contains(&vpn_config.tag){
                for route in &routes{
                    if vpn_config.do_for_tags.contains(&route.tag){
                        let mut route = route.clone();
                        route.tag = vpn_config.tag.clone();
                        route.data_traversals.push(ConnectionType::TOR);
                        if config.sinkparams.is_some(){
                            res.combinations_sink.push(route.clone());
                        }
                        if let Some(mullvad) = &config.mullvadbrowserparams{
                            for website in mullvad.generate_runs(){
                                res.combinations_mullvad.push((route.clone(),website));
                            }
                        }
                    }
                }
            }
        }
        res
    }

    async fn run_mullvad(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) 
    -> Result<(),TrafficStarError>{
        sinfo!("Creating tor connection!");
        let (route,website) = &self.combinations_mullvad[sample % self.combinations_mullvad.len()].clone();
        let mut route = route.clone();

        let interface_name = route.interface_name.clone();
        let config_device = TorInterfaceConfig::new(TrafficStarInterfaceName::from_str(&interface_name)?).await;
        let handler = TorProcess::new(config_device).await?;

        route.interface_name = handler.name().to_string();
        route.ipv4 = handler.reservation.ipv4_addr.get_ip().to_string();
        sinfo!("Tor device created, name : {}, binding interface : {}",route.interface_name, interface_name);
        let result = TrafficStarTestSession::run_browser(website, route, endhost, output_directory, trafficstar_interface::LinkType::TOR, prefix).await;
      
        sdebug!("Finished!");
        handler.stop().await?;
        sdebug!("Killed handler!");
        result?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn run_sink(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) 
    -> Result<(),TrafficStarError>{
          sinfo!("Creating tor connection!");
        let route = &self.combinations_sink[sample % self.combinations_sink.len()].clone();
        let mut route = route.clone();

        let interface_name = route.interface_name.clone();
        let config_device = TorInterfaceConfig::new(TrafficStarInterfaceName::from_str(&interface_name)?).await;
        let handler = TorProcess::new(config_device).await?;

        route.interface_name = handler.name().to_string();
        route.ipv4 = handler.reservation.ipv4_addr.get_ip().to_string();
        sinfo!("Tor device created, name : {}, binding interface : {}",route.interface_name, interface_name);
        let result = TrafficStarTestSession::run_sink( self.config.sinkparams.clone().unwrap(),route, &endhost, output_directory,prefix).await;
      
        sdebug!("Finished!");
        handler.stop().await?;
        sdebug!("Killed handler!");
        result?;
        Ok(())
    }
}

#[async_trait]
impl TrafficStarTestHandlersTraits for TorTestHandler{
    async fn run_sample(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) -> Result<(), TrafficStarError> {
        if self.combinations_sink.len()*self.config.test_parameters.samples > sample{
            self.run_sink(sample, output_directory, endhost, prefix).await
        }else{
            self.run_mullvad(sample-self.combinations_sink.len()*self.config.test_parameters.samples, output_directory, endhost, prefix).await
        }
    }

    fn total_samples(&self) -> usize {
       self.config.test_parameters.samples*(self.combinations_sink.len() + self.combinations_mullvad.len())
    }
    
    fn name(&self) -> &str {
        "Tor"
    }
    
    fn config(&self) -> &TrafficStarTestConfigFile {
        &self.config
    }

    fn uuid(&self) -> &Uuid{
        &self.uuid
     }
}