use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use trafficstar_connections::trafficstar_data_traversel_types::ConnectionType;
use trafficstar_connections::trafficstar_data_route::DataRoute;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{panicerror, sdebug, sinfo, swarn};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_mullvad::trafficstar_async_mullvad_handler::AsyncMullvadHandler;
use trafficstar_mullvad::trafficstar_mullvad_config::MullvadRelayConfig;
use trafficstar_mullvad::trafficstar_mullvad_relays::{TrafficStarMullvadRelay, WgPeerPair};
use trafficstar_processes::trafficstar_mullvad_browser::MullvadBrowserRun;
use uuid::Uuid;
use crate::trafficstar_test::{TrafficStarTestHandlersTraits, TrafficStarTestSession};
use crate::trafficstar_test_config_file::TrafficStarTestConfigFile;



#[allow(dead_code)]
#[derive(Clone, StructLoggerName)]
pub struct MullvadTestHandler{
    combinations_sink : Vec<(TrafficStarMullvadRelay,Option<TrafficStarMullvadRelay>, DataRoute)>,

    combinations_mullvad : Vec<(TrafficStarMullvadRelay,Option<TrafficStarMullvadRelay>, DataRoute, MullvadBrowserRun)>,
    pub config : TrafficStarTestConfigFile,
    routes : Vec<DataRoute>,
    handler : Arc<AsyncMullvadHandler>,
    uuid : Uuid
}

impl MullvadTestHandler {
    pub async fn new(config : TrafficStarTestConfigFile,routes : Vec<DataRoute>, config_relays : Vec<MullvadRelayConfig>
    ) -> Result<Self, TrafficStarError>{
        let handler = Arc::new(AsyncMullvadHandler::singleton().await?);
        let relays = handler.get_relays();
        let mut combinations_mullvad: Vec<(TrafficStarMullvadRelay,Option<TrafficStarMullvadRelay>,DataRoute,MullvadBrowserRun)> = Vec::new();
        let mut combinations_iperf3: Vec<(TrafficStarMullvadRelay,Option<TrafficStarMullvadRelay>,DataRoute)> = Vec::new();

        let config_do_for_tag: &Vec<String> = &config.do_for_tag;
        for relay_config in &config_relays{
            let tag = relay_config.tag.clone();

            if !config_do_for_tag.contains(&tag){
                continue;
            }
            
            for route in routes.iter(){
                if relay_config.do_for_tags.iter().any(|e| e.eq(&route.tag)){
                    for entry in relays.hostname(&relay_config.entry){
                        let mut new_route = route.clone();
                        if let Some(exits) =relay_config.exit.clone(){
                            new_route.data_traversals.push(ConnectionType::DoubleTunnel);
                            for exit in relays.hostname(&exits){
                                if entry.hostname != exit.hostname{
                                    if let Some(browser_runs) = &config.mullvadbrowserparams{
                                        for run in browser_runs.generate_runs(){
                                            combinations_mullvad.push((entry.clone(),Some(exit.clone()),new_route.clone(),run));
                                        }
                                    }
                                    if config.sinkparams.is_some(){
                                        combinations_iperf3.push((entry.clone(),Some(exit.clone()),new_route.clone()));
                                    }
                                    
                                }else{
                                    swarn!("Skipped {}-{} since trying to double jump to itself!",entry.hostname,exit.hostname);
                                }
                            }
                        }else{
                            new_route.data_traversals.push(ConnectionType::Tunnel);
                            if let Some(browser_runs) = &config.mullvadbrowserparams{
                                for run in browser_runs.generate_runs(){
                                    combinations_mullvad.push((entry.clone(),None,new_route.clone(),run));
                                }
                            }
                            if config.sinkparams.is_some(){
                                combinations_iperf3.push((entry.clone(),None,new_route.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(MullvadTestHandler {
            combinations_mullvad,
            combinations_sink: combinations_iperf3,
            config,
            routes,
            handler,
            uuid : Uuid::new_v4()
        })
    }


    async fn run_mullvad(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) 
    -> Result<(),TrafficStarError>{

        let (entry,exit,route, website) = &self.combinations_mullvad[sample % self.combinations_mullvad.len()];
        let exit = exit.clone();
        sinfo!("Entry : {}, exit : {}, interface : {}, website : {}",entry.hostname, match exit.clone() {
            Some(v) => {
                v.hostname.clone()
            }
            None => {
                "None".to_string()
            }
        }, route.interface_name, website.website);

        let wg_pair = WgPeerPair{
            entry : entry.clone(),
            exit : exit.clone()
        };
        
        let mut route = route.clone();
        let wgpeer = match entry.to_peer(exit.clone()){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::msg(format!("Received error : {}",err)))
,
        };

        route.info = Some(match serde_json::to_string(&wg_pair){
            Ok(v) => v,
            Err(err) => {
                panicerror!("Couldn't create route info peer, error : {}",err)
            },
        });
        sinfo!("Creating device!");
        let vpn_connection = self.handler.get_device().await?;
        
        vpn_connection.use_peer(wgpeer, route.fwmark).await?;

        sinfo!("Created device {}!",vpn_connection.device().name); 
        route.interface_name = vpn_connection.interface_name().to_string();
        route.ipv4 = vpn_connection.device().get_ipv4()?.to_string();
        

        let result = TrafficStarTestSession::run_browser(website, route, endhost, output_directory, trafficstar_interface::LinkType::MULLVAD, prefix).await;
        
        sdebug!("Deleting VPN connection {}",vpn_connection.device().name);
        vpn_connection.delete().await?;
        sinfo!("Finished!");
        result?;
        Ok(())
    }

    async fn run_sink(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) 
    -> Result<(),TrafficStarError>{
        
        let (entry,exit,route) = &self.combinations_sink[sample % self.combinations_sink.len()];
        let exit = exit.clone();
        sinfo!("Entry : {}, exit : {}, interface : {}",entry.hostname, match exit.clone() {
            Some(v) => {
                v.hostname.clone()
            }
            None => {
                "None".to_string()
            }
        }, route.interface_name);

        let wg_pair = WgPeerPair{
            entry : entry.clone(),
            exit : exit.clone()
        };
        
        let mut route = route.clone();
        let wgpeer = match entry.to_peer(exit.clone()){
            Ok(v) => v,
            Err(err) => return Err(TrafficStarError::msg(format!("Received error : {}",err)))
,
        };

        route.info = Some(match serde_json::to_string(&wg_pair){
            Ok(v) => v,
            Err(err) => {
                panicerror!("Couldn't create route info peer, error : {}",err)
            },
        });
        sinfo!("Creating device!");
        let vpn_connection = self.handler.get_device().await?;
        
        vpn_connection.use_peer(wgpeer, route.fwmark).await?;

        sinfo!("Created device {}!",vpn_connection.device().name); 
        route.interface_name = vpn_connection.interface_name().to_string();
        route.ipv4 = vpn_connection.device().get_ipv4()?.to_string();
        

        let result = TrafficStarTestSession::run_sink(self.config.sinkparams.clone().unwrap(), route, &endhost, output_directory,prefix).await;
        
        sdebug!("Deleting VPN connection {}",vpn_connection.device().name);
        vpn_connection.delete().await?;
        sinfo!("Finished!");
        result?;
        Ok(())
    }
    
    
}

#[async_trait]
impl TrafficStarTestHandlersTraits for MullvadTestHandler{
    
    
    async fn run_sample(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) -> Result<(), TrafficStarError> {
        if self.combinations_sink.len()*self.config.test_parameters.samples > sample{
            sdebug!("{}",self.combinations_sink.len());
            self.run_sink(sample, output_directory, endhost, prefix).await
        }else{
            self.run_mullvad(sample-self.combinations_sink.len()*self.config.test_parameters.samples, output_directory, endhost, prefix).await
        }
        
    }

    fn total_samples(&self) -> usize {
       
        (self.combinations_mullvad.len()+self.combinations_sink.len())*self.config.test_parameters.samples
    }
    
    fn name(&self) -> &str {
        "Mullvad"
    }
    
    fn config(&self) -> &TrafficStarTestConfigFile {
        &self.config
    }

     fn uuid(&self) -> &Uuid{
        &self.uuid
     }
}
