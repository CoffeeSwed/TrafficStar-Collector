use std::sync::Arc;

use clap::ArgMatches;
use trafficstar_logger::{serror, sinfo, swarn};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_mullvad::{trafficstar_async_mullvad_handler::AsyncMullvadHandler, trafficstar_mullvad_config::MullvadRelayConfig};
use trafficstar_tests::{trafficstar_test::{TrafficStarTestHandler, TrafficStarTestSession}, trafficstar_test_config_file::TrafficStarTestConfigFile, trafficstar_tests_mullvad::MullvadTestHandler, trafficstar_tests_pure::TestHandlerPure, trafficstar_tests_tor::TorTestHandler};
use trafficstar_tor::tor_process::tor_vpn_config::TorVPNConfig;
use trafficstar_utilities::{create_single_runtime, get_singleton_multi};
use trafficstar_connections::trafficstar_data_route::DataRoute;
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::configurations::Configuration;

#[derive(StructLoggerName)]
pub struct Client{
    configuration : Configuration,
    standard_interfaces : Vec<DataRoute>,
    test_sessions : Vec<TrafficStarTestConfigFile>,
    tor_generators : Vec<TorVPNConfig>,
    mullvad_generators : Vec<MullvadRelayConfig>,
    
}

impl Client{
    pub fn new(matches : ArgMatches) -> Result<Self, TrafficStarError>{
        let mut result = Self { 
            configuration : Configuration::try_from(matches)?, 
            standard_interfaces : Vec::new(),
            test_sessions : Vec::new(),
            tor_generators : Vec::new(),
            mullvad_generators : Vec::new(),
        };
        if result.configuration.test_session_params.is_none(){
            return Err(TrafficStarError::msg("Missing required variable testparams".into()))
        }
        let interfaces = result.configuration.interfaces.as_path();
        if !interfaces.exists() || !interfaces.is_dir(){
            return Err(TrafficStarError::msg("Interfaces directory specified doesn't exist or is not a directory!".into()))
        }
        let found_interfaces = trafficstar_utilities::trafficstar_networking::get_interfaces()?;

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
                        if found_interfaces.iter().position(|x| x.as_string().eq(&v.interface_name)).is_some(){                
                            sinfo!("Append interface file {:?}, entry : {}!",file.path(),v);
                            result.standard_interfaces.push(v);
                        }else{
                            serror!("Ingnoring Interface file {:?} as it's interface {} is not found!",file.path(), v.interface_name);
                        }
                    },
                    Err(err) => {
                        swarn!("Failed reading interface file {:?}, error : {}",file.path(), err);
                    },
                };
            }else{  
                swarn!("Ignoring direntry  {:?} as it's not a file!",file.path());
            }
        let testparams = match &result.configuration.test_session_params{
            Some(v) => v.as_path(),
            None => return Err("Missing required variable testparams!".into()),
        };
        if !testparams.exists() || !testparams.is_dir(){
            return Err(TrafficStarError::msg("Test session directory specified doesn't exist or is not a directory!".into()))
        }
        
        for file in std::fs::read_dir(testparams)?{
            let file = match file{
                Ok(v) => v,
                Err(err) => {
                    serror!("Error reading file : {}!",err);
                    continue;
                },
            };

            if file.path().is_file(){
                match TrafficStarTestConfigFile::from_json(&file.path()){
                    Ok(v) => {
                        if let Some(tests_names) = &result.configuration.torun{
                            if let Some(name) = &v.test_parameters.name{
                                if tests_names.iter().find(|e| e.eq_ignore_ascii_case(name.as_str())).is_some(){
                                    sinfo!("Append test session file {:?}, entry : {}!",file.path(),v);
                                    result.test_sessions.push(v);
                                }else{
                                    swarn!("Read test session file {:?}, entry : {} but ignored it since it didn't match any of the test names to run",file.path(),v);
                                }
                            }else{
                                swarn!("Read test session file {:?}, entry : {} but ignored since it didn't match any of the test names to run",file.path(),v);

                            }
                        }else{
                            sinfo!("Append test session file {:?}, entry : {}!",file.path(),v);
                            result.test_sessions.push(v);
                        }
                    },
                    Err(err) => {
                        swarn!("Failed reading test session file {:?}, error : {}",file.path(), err);
                    },
                };
            }else{ 
                //if file.file_name().eq_ignore_ascii_case("generators"){
                  //  sinfo!("Found generator directory for mullvad : {:?}",file.path());
                //}else{
                swarn!("Ignoring direntry  {:?} as it's not a file and doesn't contain generators!",file.path());
                //}
            }
        }
        }

        if let Some(tor_configs) = &result.configuration.tor_generators{
            if !tor_configs.exists() || !tor_configs.is_dir(){
                return Err(TrafficStarError::msg("Tor connection generator directory specified doesn't exist or is not a directory!".into()))
            }
            for file in std::fs::read_dir(tor_configs)?{
            let file = match file{
                Ok(v) => v,
                Err(err) => {
                    serror!("Error reading file : {}!",err);
                    continue;
                },
            };

            if file.path().is_file(){
                match TorVPNConfig::from_json(&file.path()){
                    Ok(v) => {
                        sinfo!("Append tor connection generator file {:?}, entry : {}!",file.path(),v);
                        result.tor_generators.push(v);
                    },
                    Err(err) => {
                        swarn!("Failed tor connection generator file {:?}, error : {}",file.path(), err);
                    },
                };
            }else{  
                swarn!("Ignoring direntry  {:?} as it's not a file!",file.path());
            }
        }
        }

        if let Some(mullvad_configs) = &result.configuration.mullvad_generators{
            if !mullvad_configs.exists() || !mullvad_configs.is_dir(){
                return Err(TrafficStarError::msg("Mullvad connection generator directory specified doesn't exist or is not a directory!".into()))
            }
            for file in std::fs::read_dir(mullvad_configs)?{
            let file = match file{
                Ok(v) => v,
                Err(err) => {
                    serror!("Error reading file : {}!",err);
                    continue;
                },
            };

            if file.path().is_file(){
                match MullvadRelayConfig::from_json(&file.path()){
                    Ok(v) => {
                        sinfo!("Append mullvad connection generator file {:?}, entry : {}!",file.path(),v);
                        result.mullvad_generators.push(v);
                    },
                    Err(err) => {
                        swarn!("Failed mullvad connection generator file {:?}, error : {}",file.path(), err);
                    },
                };
            }else{  
                swarn!("Ignoring direntry  {:?} as it's not a file!",file.path());
            }
        }

        }
        

        Ok(result)
    }

    pub fn run(&self) -> Result<(),TrafficStarError>{
        sinfo!("Starting program!");
        if let Some(accounts) = &self.configuration.mullvad_accounts{
            get_singleton_multi().block_on(async {
                if let Ok(handler) = AsyncMullvadHandler::singleton().await{
                    for i in accounts{
                        if let Err(err) = handler.add_account(i.clone()).await{
                            serror!("Failed adding mullvad account, error :  {}!",err);
                        }else{
                            sinfo!("Added an mullvad account!");
                        }
                    }
                }
            })
        };
        for config in &self.test_sessions{
            sinfo!("Performing test session : {}",config.test_parameters.name.clone().unwrap_or("Nameless".into()));
            create_single_runtime()?.block_on(async move{
            let executors : Vec<Arc<Box<TrafficStarTestHandler>>> = vec![
                Arc::new(Box::new(MullvadTestHandler::new(config.clone(), self.standard_interfaces.clone(),self.mullvad_generators.clone()).await?)),
                Arc::new(Box::new(TorTestHandler::new(config.clone(), self.standard_interfaces.clone(), self.tor_generators.clone()))),
                Arc::new(Box::new(TestHandlerPure::new(config.clone(), self.standard_interfaces.clone())))
            ];
            let mut test_session = TrafficStarTestSession::new(executors, self.configuration.storage.clone());
            test_session.run(self.configuration.addr.0.clone()+":"+&self.configuration.addr.1.to_string(), self.configuration.directory_prefix.clone());
            
            Ok::<(),TrafficStarError>(())
            })?;
        }
        Ok(())
    }
}