pub mod trafficstar_mullvad_config;
pub mod trafficstar_mullvad_device;

mod trafficstar_async_mullvad_account;
pub mod trafficstar_mullvad_relays;
pub mod trafficstar_mullvad_requests;
pub mod trafficstar_async_mullvad_handler;
mod trafficstar_async_mullvad_handler_structs;
mod trafficstar_async_mullvad_slave;


#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::{ Arc, Once};

    use crate::trafficstar_async_mullvad_handler::{AsyncMullvadHandler};

    use log::{error, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::TrafficStarLogger;
    use trafficstar_utilities::{get_multi_runtime, run_command};
    use trafficstar_utilities::trafficstar_pipes::TrafficStarPipePairAsync;



    const _STRING_WIREGUARD_MULLVAD_INFO: &str = "mullvad.json";

    const MULLVAD_ACCOUNT_ENV : &str = "TRAFFICSTAR_MULLVAD_ACCOUNT";

    #[warn(unused_unsafe)]
    pub fn setup() {
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res {
                log::set_max_level(log::LevelFilter::Debug);
            }
        });
    }

   


    #[tokio::test]
    async fn test_hostnames(){
        setup();
        let rt: tokio::task::JoinHandle<Result<(),TrafficStarError>> = tokio::task::spawn(async move {
            let hostname_one = r"(se-got-wg-003|es-mad-wg-102|no-osl-wg-102|us-phx-wg-208|fr-bod-wg-002|ca-mtr-wg-304|ie-dub-wg-103|jp-osa-wg-004|ar-bue-wg-002|bg-sof-wg-001|be-bru-wg-101)";
            let handler = Arc::new(AsyncMullvadHandler::singleton().await?);
            let relays = handler.get_relays();
            let matches_one = relays.hostname(hostname_one);
            info!("Matches [{}]: {} {:?}",hostname_one,matches_one.len(),matches_one);
            
            
            Ok(())
        });
        if let Err(err) = rt.await.unwrap(){
            error!("ERROR : {}",err);
        }else{
            info!("Hostnames ran like expected!");
        }
    }



    #[tokio::test]
    async fn random_hosts(){
        setup();
        let rt: tokio::task::JoinHandle<Result<(),TrafficStarError>> = tokio::task::spawn(async move {
            let handler = Arc::new(AsyncMullvadHandler::singleton().await?);
            let relays = handler.get_relays();
            for relay in relays.random_hosts(10, 6732348)?{
                info!("Relay : {}",relay.hostname);
            }
            Ok(())
        });
        if let Err(err) = rt.await.unwrap(){
            error!("ERROR : {}",err);
        }else{
            info!("Hostnames ran like expected!");
        }
    }
    
    
    /*#[test]
    fn get_devices() {
        setup();
        let handler = trafficstar_mullvad_handler::MullvadHandler::get_singleton();
        let results = handler.get_devices();
        if let Ok(devices) = results {
            for device in devices {
                info!("Read device {}!", device.name);
                info!("Deleting device {}", device.name);
                let deleted = handler.delete_device(&device);
                assert_eq!(
                    deleted.is_ok(),
                    true,
                    "Couldn't delete device {}, got error {}!",
                    device.name,
                    deleted.err().unwrap()
                )
            }
        } else {
            assert_eq!(
                results.is_ok(),
                true,
                "Received following error when trying to get devices : {}!",
                results.err().unwrap()
            )
        }
        info!("Creating device!");
        let created = handler.create_device(&WireguardKeys::default()).unwrap();
        assert_eq!(
            created.is_ok(),
            true,
            "Couldn't create device, received following error : {}!",
            created.err().unwrap()
        );
        let results = handler.get_devices();
        if let Ok(devices) = results {
            assert_eq!(
                devices.is_empty(),
                false,
                "Devices was empty when not expected, when trying to create device the following
            response was gotten : {}!",
                created.unwrap().text().unwrap()
            );
            for device in devices {
                info!("Read device {}!", device.name);
                info!("Deleting device {}", device.name);
                let deleted = handler.delete_device(&device);
                assert_eq!(
                    deleted.is_ok(),
                    true,
                    "Couldn't delete device {}, got error {}!",
                    device.name,
                    deleted.err().unwrap()
                )
            }
        } else {
            assert_eq!(
                results.is_ok(),
                true,
                "Received following error when trying to get devices : {}!",
                results.err().unwrap()
            )
        }
    }*/


    async fn test_mullvad_create(handler : Arc<AsyncMullvadHandler>) -> Result<(),TrafficStarError>{
        
        let deviceholder = match handler.get_device().await{
        Ok(v) => v,
            Err(err) => {
                error!("Received error : {}!",err);
                return Err(err);
            },
        };
        let device = deviceholder.device().clone();
        info!("Got device {}!",device.name);
        
        let peer = handler.get_relays().hostname("se-got-wg-001")[0].to_peer(None).unwrap();
        deviceholder.use_peer(peer,None).await?;

        /*
        info!("wg-show : {}",String::from_utf8(
            run_command("wg", vec!["show",deviceholder.interface_name()]).unwrap().stdout).unwrap()
        );

         info!("ip link show : {}",String::from_utf8(
            run_command("ip", vec!["link","show",deviceholder.interface_name()]).unwrap().stdout).unwrap()
        );

        info!("ip --brief a : {}",String::from_utf8(
            run_command("ip", vec!["--brief","a"]).unwrap().stdout).unwrap()
        );*/
        

        info!("curl test : {}",String::from_utf8(
            run_command("curl", vec!["https://api.ipify.org/","--interface",deviceholder.interface_name()]).unwrap().stdout).unwrap()
        );
        
        info!("Dropping device {}!",device.name);
        drop(deviceholder);
        info!("Dropped device {}!",device.name);
        Ok(())
    }
    
    #[tokio::test]
    async fn mullvad_async_test(){
        setup();
        let handler_org = match AsyncMullvadHandler::singleton().await{
            Ok(v) => Arc::new(v),
            Err(err) => {error!("Failed to create AsyncMullvadHandler, err : {}",err);
            return;
            },
        };
        match handler_org.add_account(env::var(MULLVAD_ACCOUNT_ENV).unwrap()).await{
            Ok(_) => {},
            Err(err) => {
                error!("Failed to add account, error : {}",err);
                return;
            },
        };
        let mut threads:Vec<std::thread::JoinHandle<Result<(), TrafficStarError>>>  = Vec::new();
        info!("Created handler!");
        
        info!("Getting device!");
        for i in 0..3{
            let handler = handler_org.clone();
            threads.push(
            std::thread::spawn(move || {
                TrafficStarLogger::add_nick_thread(i.to_string());
                TrafficStarLogger::set_threadhook_nick(TrafficStarLogger::get_nick_thread());

                let rt = get_multi_runtime()?;
                let future = rt.spawn(async move {
                    
                    test_mullvad_create(handler).await
                });
                rt.block_on(future).unwrap()
                })
            );
            
        }
        for (index, future) in threads.into_iter().enumerate(){
            info!("Joining {}!",index);
            let p = future.join().unwrap();
            info!("Joined {}!",index);
            if let Err(err) = p{
                error!("Error for {} : {}",index,err);
            }
        }
        info!("Joined all threads!");

        match handler_org.kill().await{
            Ok(_) => info!("Killed handler as we should!"),
            Err(err) => error!("Failed to kill handler, Err : {}",err),
        };
        
        let pipes = TrafficStarPipePairAsync::new_pairs().await.unwrap();
        drop(pipes);
        
    }

}
