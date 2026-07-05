
use std::path::PathBuf;

use async_trait::async_trait;
use trafficstar_connections::trafficstar_data_route::DataRoute;
use trafficstar_errors::{traffic_star_error::TrafficStarError};
use trafficstar_logger::{sdebug, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_processes::trafficstar_mullvad_browser::MullvadBrowserRun;
use uuid::Uuid;


use crate::{trafficstar_test::{TrafficStarTestHandlersTraits, TrafficStarTestSession}, trafficstar_test_config_file::TrafficStarTestConfigFile};
#[derive(StructLoggerName)]
pub struct TestHandlerPure{
    combinations_sinks : Vec<DataRoute>,
    combinations_mullvad : Vec<(DataRoute,MullvadBrowserRun)>,

    pub config : TrafficStarTestConfigFile,
    uuid : Uuid
}

impl TestHandlerPure{
    pub fn new(config : TrafficStarTestConfigFile, routes : Vec<DataRoute>) -> Self{
        let mut res = Self{
            combinations_sinks : Vec::new(),
            combinations_mullvad : Vec::new(),
            config : config.clone(),
            uuid : Uuid::new_v4()
        };
        for route in &routes{
                if config.do_for_tag.contains(&route.tag){
                    let route = route.clone();
                    if config.sinkparams.is_some(){
                        res.combinations_sinks.push(route.clone());
                    }
                    if let Some(mullvad) = &config.mullvadbrowserparams{
                        for website in mullvad.generate_runs(){
                            res.combinations_mullvad.push((route.clone(),website));
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

        let result = TrafficStarTestSession::run_browser(website, route.clone(), endhost, output_directory, trafficstar_interface::LinkType::TOR, prefix).await;
      
        sdebug!("Finished!");
        result?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn run_sink(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) 
    -> Result<(),TrafficStarError>{
          sinfo!("Creating tor connection!");
        let route = &self.combinations_sinks[sample % self.combinations_sinks.len()].clone();

    
        let result = TrafficStarTestSession::run_sink( self.config.sinkparams.clone().unwrap(),route.clone(), &endhost, output_directory,prefix).await;
      
        sdebug!("Finished!");
        result?;
        Ok(())
    }
}

#[async_trait]
impl TrafficStarTestHandlersTraits for TestHandlerPure{
    async fn run_sample(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) -> Result<(), TrafficStarError> {
        if self.combinations_sinks.len()*self.config.test_parameters.samples > sample{
            self.run_sink(sample, output_directory, endhost, prefix).await
        }else{
            self.run_mullvad(sample-self.combinations_sinks.len()*self.config.test_parameters.samples, output_directory, endhost, prefix).await
        }
    }

    fn total_samples(&self) -> usize {
       self.config.test_parameters.samples*(self.combinations_sinks.len() + self.combinations_mullvad.len())
    }
    
    fn name(&self) -> &str {
        "Pure"
    }
    
    fn config(&self) -> &TrafficStarTestConfigFile {
        &self.config
    }

    fn uuid(&self) -> &Uuid{
        &self.uuid
     }
}