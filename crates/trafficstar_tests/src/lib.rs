pub mod trafficstar_test_config_file;
pub mod trafficstar_tests_mullvad;
pub mod trafficstar_tests_tor;
pub mod trafficstar_test_configuration;
pub mod trafficstar_test;
pub mod trafficstar_tests_pure;

pub mod trafficstar_test_errors;

#[cfg(test)]
mod tests {
    const ENV_MULLVAD_BROWSER_SCRIPT : &str = "TRAFFICSTAR_WEBSCRIPT";
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::thread::JoinHandle;
    use std::time::Duration;
    use std::{env};
    use std::{fs::File, io::Write, sync::Once};
    use log::{error, info};
    use trafficstar_connections::trafficstar_data_route::DataRoute;
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::{panicerror, trafficstar_logger::TrafficStarLogger};
    use trafficstar_mullvad::trafficstar_async_mullvad_handler::AsyncMullvadHandler;
    use trafficstar_mullvad::trafficstar_mullvad_config::MullvadRelayConfig;
    use trafficstar_nodes::trafficstar_node_server::{ServerNode};

    use trafficstar_tester::replace_create_temp_dir;
    use trafficstar_processes::trafficstar_mullvad_browser::MullvadBrowserSettings;
    use trafficstar_tor::tor_process::tor_vpn_config::TorVPNConfig;
    use trafficstar_utilities::sink::settings::SinkSenderSettings;
    use trafficstar_utilities::{async_fetch_public_ip, get_multi_runtime};
    
    use crate::trafficstar_test::TrafficStarTestSession;
    use crate::trafficstar_test_config_file::{TrafficStarTestConfigFile, TrafficStarTestParameters};
    use crate::trafficstar_tests_mullvad::MullvadTestHandler;
    use crate::trafficstar_tests_tor::TorTestHandler;

    #[warn(unused_unsafe)]
    pub fn setup(){
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res{
                log::set_max_level(log::LevelFilter::Debug);
                
            }
            TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![] }));
            TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![] }));
        });


    }

    #[test]
    fn config_parser_test(){
        setup();
        let config_vpns = TrafficStarTestConfigFile{
            test_parameters : TrafficStarTestParameters{
                name: Some("vpns".into()),
                samples: 100,
                parallel: 200,

            },
            sinkparams: Some(SinkSenderSettings{
                time: 5,
                /*bandwidth: Some("1M".into()),
                num: None,
                i: None,
                k: None,
                udp: None,
                length: None,
                window: None,
                c: None,
                allowed_droppeds : None*/
            }),
            mullvadbrowserparams : Some(
                MullvadBrowserSettings { 
                    websites: vec!["https://youtube.se".into()],
                    test_script : PathBuf::from_str(&env::var("TRAFFICSTAR_MULLVAD_ACCOUNT").unwrap()).unwrap(),
                    rate : Some("8mbit".into()),
                    max_time : Duration::from_secs(32),
                }),

            do_for_tag: vec!["onehop".to_string(), "twohop".to_string()],
        };
        let config_tor = TrafficStarTestConfigFile{
            test_parameters : TrafficStarTestParameters{
                name: Some("tor".into()),
                samples: 100,
                parallel: 200,

            },
            sinkparams: Some(SinkSenderSettings{
                time: 1,
                /*bandwidth: Some("2M".into()),
                num: Some("3".into()),
                i: Some(4.0),
                k: Some("5".into()),
                udp: Some(false),
                length: Some("wow".into()),
                window: Some("wow".into()),
                c: Some("reno".into()),
                allowed_droppeds: None,*/
            }),
             mullvadbrowserparams : Some(
                MullvadBrowserSettings { 
                    websites: vec!["https://facebook.se".into()],
                    test_script : PathBuf::from_str(ENV_MULLVAD_BROWSER_SCRIPT).unwrap(),
                rate : Some("8mbit".into()),
                    max_time : Duration::from_secs(3600),
                }),
            do_for_tag: vec!["tor".into()],
        };
        let configs_original = vec![config_tor.clone(),config_vpns.clone()];
        info!("Config vpns is : {}",config_vpns);
        info!("Config tor is : {}",config_tor);

        let dir = match replace_create_temp_dir("trafficstar_tests_tests") {
            Ok(v) => v,
            Err(err) => panicerror!("Couldn't create temp dir, error :{}",err),
        };
        for config in configs_original.as_slice(){
            let mut file = match File::create(dir.join(&(config.test_parameters.clone().name.clone().unwrap_or("None".into())+".json"))){
                Ok(v) => v,
                Err(err) =>panicerror!("Couldn't create file, reason {}",err),
            };
            let config_string = match serde_json::to_string(&config){
                Ok(v) => v,
                Err(err) => panicerror!("Couldn't serialize, error {}",err),
            };
            info!("serialized config : {}",config_string);
            match file.write_all(config_string.as_bytes()){
                Ok(_) => {},
                Err(err) => panicerror!("Couldnt write to file, reason : {}",err),
            }
            info!("Wrote to file!");
        }
        info!("Getting configs from directory!");
        let new_configs = match TrafficStarTestConfigFile::from_dirs_json(&dir){
            Ok(v) => v,
            Err(err) => panicerror!("Failed to read config files, error : {}",err),
        };
        info!("Read configs from directory!");
        assert_eq!(new_configs.len(), configs_original.len(),"Didn't read all data as expected!");
        for config_saved in new_configs{
            let config_saved = match config_saved{
                Ok(c) => c,
                Err(err) => panicerror!("Couldn't read json, reason : {}",err),
            };
            match configs_original.clone().iter().find(|&x| *x == config_saved){
                Some(_v) => {
                    info!("{} does match one of the originals!",config_saved);
                },
                None => {
                    panicerror!("{} does not match any of the original configs!",config_saved);
                },
            }
        }



    }

    fn run_nodes_server(listener : TcpListener) -> Result<JoinHandle<()>, TrafficStarError>{
        let route = DataRoute::from_default_interface()?;
        info!("Created DataRoute!");

        let node = ServerNode::init_server(listener, route, 
            PathBuf::from_str("/home/root/saved/server").unwrap());
        info!("Created server node!");

        Ok(node.run_server( false))
    }

    async fn mullvad_test_run(config_vpns : TrafficStarTestConfigFile, 
        routes : Vec<DataRoute>, 
        config_relays : Vec<MullvadRelayConfig>) -> Result<(),TrafficStarError>{
        let generator = Box::new(match MullvadTestHandler::new(
            config_vpns.clone(), 
            routes, 
            config_relays
        ).await{
            Ok(v) => {
                v
            }
            Err(err) => {panicerror!("Couldn't create generator, error : {}",err)},
        });

        let mut test_session = TrafficStarTestSession::new(
            vec![Arc::new(generator)], PathBuf::from_str("/home/root/saved/client").unwrap());
        info!("Created TrafficStarTestSession : {}",test_session);
        info!("Running!");
        
        test_session.run(async_fetch_public_ip().await+":5201", Some("TEST".into()));

        Ok(())
    }

    #[test]
    fn mullvad_test(){
        setup();
        let route : DataRoute = match DataRoute::from_default_interface(){
            Ok(v) => v,
            Err(err) => panicerror!("Couldn't create route, error : {}",err),
        };
        let relay_config_1 = MullvadRelayConfig{
            tag : "onehop".into(),
            entry : r"(se-got-wg-003|es-mad-wg-102|no-osl-wg-102|us-phx-wg-208|fr-bod-wg-002|ca-mtr-wg-304|ie-dub-wg-103|jp-osa-wg-004|ar-bue-wg-002|bg-sof-wg-001|be-bru-wg-101)".into(),
            exit : None,
            do_for_tags : vec![route.tag.clone()]
        };

        let config_vpns = TrafficStarTestConfigFile{
            test_parameters : TrafficStarTestParameters{
                name: Some("vpns".into()),
                samples: 1,
                parallel: 1,
            },
            sinkparams: Some(SinkSenderSettings{
                time: 30,
            }),
            mullvadbrowserparams : Some(
                MullvadBrowserSettings { websites: vec!["https://northernaurora.org".into(),"https://google.com".into()],
                test_script : PathBuf::from_str(&env::var("TRAFFICSTAR_WEBSCRIPT").unwrap()).unwrap(),
                rate : None,
                max_time : Duration::from_secs(60*2+20),
             }
            ),
            do_for_tag: vec!["onehop".to_string()],
        };
        info!("Creating generator!");

        let listener = TcpListener::bind("0.0.0.0:5201").unwrap();
        let _addr = listener.local_addr().unwrap();
        let server = match run_nodes_server(listener){
            Ok(v) => v,
            Err(err) => {
                error!("Failed to start server, reason : {}",err);
                return;
            },
        };
        if let Err(err) = std::thread::spawn(move || {
           get_multi_runtime().unwrap().block_on(async move{
                AsyncMullvadHandler::singleton().await.unwrap().add_account(env::var("TRAFFICSTAR_MULLVAD_ACCOUNT").unwrap()).await?;
                                AsyncMullvadHandler::singleton().await.unwrap().add_account(env::var("TRAFFICSTAR_MULLVAD_ACCOUNT").unwrap()).await?;

                mullvad_test_run(config_vpns,vec![route],vec![relay_config_1]).await
           })
        }).join().unwrap(){
            error!("Error {}!",err);
        }

    
        drop(server);
    }


    async fn tor_test_run(config_vpns : TrafficStarTestConfigFile, 
        routes : Vec<DataRoute>, vpn_config : TorVPNConfig) -> Result<(),TrafficStarError>{
            info!("{}",routes[0]);
        let generator = Box::new(TorTestHandler::new(config_vpns.clone(), 
            routes,
            vec![vpn_config]
        ));
        let mut test_session = TrafficStarTestSession::new(
            vec![Arc::new(generator)], PathBuf::from_str("/home/root/saved/client").unwrap());
        info!("Created TrafficStarTestSession : {}",test_session);
        info!("Running!");
        test_session.run(async_fetch_public_ip().await+":5201",Some("TEST".into()));

        Ok(())
    }

    #[test]
    fn tor_test(){
        setup();
        let route : DataRoute = match DataRoute::from_default_interface(){
            Ok(v) => v,
            Err(err) => panicerror!("Couldn't create route, error : {}",err),
        };

        let tor_vpn_config = TorVPNConfig{
            do_for_tags : vec![route.tag.clone()],
            tag : "tor".into(),
        };
        
        let config_tors = TrafficStarTestConfigFile{
            test_parameters : TrafficStarTestParameters{
                name: Some("tor".into()),
                samples: 1000,
                parallel: 1,
            },
           sinkparams: Some(SinkSenderSettings{
                time: 30,
            }),
            mullvadbrowserparams : Some(
                MullvadBrowserSettings { websites: vec!["https://hypr.land/".into(),"https://youtube.se/".into()],
                test_script : PathBuf::from_str(&env::var("TRAFFICSTAR_WEBSCRIPT").unwrap()).unwrap(),
                rate : None,
                max_time : Duration::from_secs(240),
             }),
            do_for_tag: vec!["tor".into()],
        };
        info!("Creating generator!");

        let listener = TcpListener::bind("0.0.0.0:5201").unwrap();
        let _addr = listener.local_addr().unwrap();
        let server = match run_nodes_server(listener){
            Ok(v) => v,
            Err(err) => {
                error!("Failed to start server, reason : {}",err);
                return;
            },
        };
        if let Err(err) = std::thread::spawn(move || {
           get_multi_runtime().unwrap().block_on(async move{
                tor_test_run(config_tors,vec![route], tor_vpn_config).await
           })
        }).join().unwrap(){
            error!("Error {}!",err);
        }
        drop(server);
    }
}