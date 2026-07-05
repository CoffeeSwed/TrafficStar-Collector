use log::{error, info};
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::controller::InterfaceController;

#[allow(clippy::unused_async)]
async fn test_controller_inner() -> Result<(),TrafficStarError>{
    let controller = InterfaceController::new()?;
    info!("Created controller!");
    for interface in controller.get_links().await?{
        info!("Name : {}",interface.as_string());
        info!("Addresses : {:?}", controller.get_ipv4_addresses(&interface).await?);
    }
    
    info!("fwmarks : {:?}",controller.get_used_fwmarks().await?);

    for rule in controller.get_rules_ip().await?{
        info!("Rule table : {}",rule.header.table);
    }

    info!("Iptables:");
    for table in controller.get_iptables(){
        info!("\t{}\n\t\tChains",table);
        for chains in controller.get_chains_iptable(&table).await.unwrap(){
            info!("\t\t\t{}",chains);
        }
        info!("\t\tRules");
        for (rule,_) in controller.get_rules_iptable(&table).await.unwrap(){
            info!("\t\t\t{}",rule);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_controller(){
    trafficstar_logger::setup_and_use();
    match tokio::spawn(test_controller_inner()).await.unwrap(){
        Ok(_) => info!("Ran like expected!"),
        Err(err) => error!("Failed, err : {}",err),
    }
}


