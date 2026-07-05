use std::{net::{Ipv4Addr, Ipv6Addr}, str::FromStr};

use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;

#[derive(Clone,Serialize,Deserialize, PartialEq)]
pub struct MullvadDevice {
    pub id: String,
    pub name: String,
    pub pubkey: String,
    pub ipv4_address: String,
    pub ipv6_address: String,
    pub hijack_dns: bool,
    pub created: String,
    pub account_number : String,
}

impl MullvadDevice{
    pub fn get_ipv4(&self) -> Result<Ipv4Addr,TrafficStarError>{
        let mut addr = self.ipv4_address.clone();
        if let Some(position) = addr.find("/"){
            let _ = addr.split_off(position);
        }
        match Ipv4Addr::from_str(&addr){
            Ok(v) => Ok(v),
            Err(err) => Err(format!("Parse error for ipv4 addr {}, {}",&addr,err).into()),
        }
    }

    pub fn get_ipv6(&self) -> Result<Ipv6Addr, TrafficStarError>{
         let mut addr = self.ipv6_address.clone();
        if let Some(position) = addr.find("/"){
            let _ = addr.split_off(position);
        }
        match Ipv6Addr::from_str(&addr){
            Ok(v) => Ok(v),
            Err(err) => Err(format!("Parse error for ipv6 addr {}, {}",&addr,err).into()),
        }
    }
}