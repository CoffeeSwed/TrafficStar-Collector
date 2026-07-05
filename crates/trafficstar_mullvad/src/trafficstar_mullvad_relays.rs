use std::io::ErrorKind;

use regex::Regex;
use serde::{Deserialize, Serialize};
use trafficstar_connections::trafficstar_wireguard::WireguardPeer;
use trafficstar_errors::traffic_star_error::TrafficStarError;

#[derive(Deserialize, Debug)]
pub struct MullvadRelaysResponse {
    pub countries: Vec<TrafficStarMullvadCountry>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TrafficStarMullvadCountry {
    pub name: String,
    pub code: String,
    pub cities: Vec<TrafficStarMullvadCities>,
}

impl TrafficStarMullvadCountry{
    pub fn hosts(&self) -> Vec<&TrafficStarMullvadRelay>{
        let mut hosts = vec![];
        for city in &self.cities{
            for host in &city.relays{
                hosts.push(host);
            }
        }
        hosts
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TrafficStarMullvadCities {
    pub name: String,
    pub code: String,
    pub latitude: f64,
    pub longitude: f64,
    pub relays: Vec<TrafficStarMullvadRelay>,
}

#[derive(Deserialize,Serialize, Debug, Clone)]
pub struct TrafficStarMullvadRelay {
    pub hostname: String,
    pub ipv4_addr_in: String,
    pub ipv6_addr_in: String,
    pub public_key: String,
    pub multihop_port: u32,
}

#[derive(Deserialize,Serialize)]
pub struct WgPeerPair{
    pub entry : TrafficStarMullvadRelay,
    pub exit : Option<TrafficStarMullvadRelay>
}

pub const MULLVAD_ONE_HOP_PORT: u16 = 51820;

impl MullvadRelaysResponse {
    fn string_matches(value: &str, poor_regex: String) -> bool {
        let glob = Regex::new(&poor_regex);
        if glob.is_err(){
            return false;
        }
        let glob = glob.unwrap();

        glob.is_match(value)
    }

    #[allow(dead_code)]
    pub fn hostname(&self, value: &str) -> Vec<&TrafficStarMullvadRelay> {
        let mut res: Vec<&TrafficStarMullvadRelay> = Vec::new();
        for country in &self.countries {
            for cities in &country.cities {
                for relay in &cities.relays {
                    if MullvadRelaysResponse::string_matches(&relay.hostname, value.to_string())
                    {
                        res.push(relay);
                    }
                }
            }
        }
        res
    }

    pub fn relays(&self) -> Vec<TrafficStarMullvadRelay> {
        let mut res: Vec<TrafficStarMullvadRelay> = Vec::new();
        for country in &self.countries {
            for cities in &country.cities {
                for relay in &cities.relays {
                    res.push(relay.clone())
                }
            }
        }
        res
    }

    pub fn random_hosts(&self, amount : usize, seed : u128) -> Result<Vec<TrafficStarMullvadRelay>, TrafficStarError>{
        let mut selected: Vec<TrafficStarMullvadRelay> = vec![];
        let mut randomizer = trafficstar_utilities::randomizer::Lcg128XRandomizer::new(
            rand_pcg::Pcg64::new(seed, 0xa02bdbf7bb3c0a7ac28fa16a64abf96));
        let mut countries = vec![];
        
        while selected.len() != amount{
            if countries.is_empty(){
                countries = self.countries.clone();
                trafficstar_utilities::shuffler::shuffle(&mut countries, &mut randomizer);
            }
            if let Some(country) = countries.pop(){
                let mut relays = country.hosts();
                trafficstar_utilities::shuffler::shuffle(&mut relays, &mut randomizer);
                loop{
                    if let Some(host) = relays.pop() 
                    &&  selected.iter().find(|x| x.hostname == *host.hostname).is_none(){
                        selected.push(host.clone());
                        break;
                    }
                }
            }else{
                return Err("Countries have the size of zero!".into())
            }
        }
        Ok(selected)
    }
}

impl TrafficStarMullvadRelay {
    pub fn to_peer(
        &self,
        other: Option<TrafficStarMullvadRelay>,
    ) -> Result<WireguardPeer, ErrorKind> {
        let result = WireguardPeer::try_from(self);

        if let Some(other) = other
            && let Ok(mut us) = result
        {
            us.endpoint = self.ipv4_addr_in.clone() + ":" + &other.multihop_port.to_string();
            us.pubkey = other.public_key;
            return Ok(us);
        }

        result
    }
}

impl TryFrom<&TrafficStarMullvadRelay> for WireguardPeer {
    type Error = ErrorKind;

    fn try_from(value: &TrafficStarMullvadRelay) -> Result<Self, Self::Error> {
        Ok(WireguardPeer {
            allowedips: "0.0.0.0/0, ::0/0".to_string(),
            endpoint: value.ipv4_addr_in.clone() + ":" + &MULLVAD_ONE_HOP_PORT.to_string(),
            pubkey: value.public_key.clone(),
        })
    }
}
