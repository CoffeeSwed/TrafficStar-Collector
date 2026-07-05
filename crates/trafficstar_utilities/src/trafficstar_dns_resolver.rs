use std::{collections::HashMap, net::Ipv4Addr, sync::Arc};

use hickory_resolver::{Resolver, config::ResolverConfig, name_server::TokioConnectionProvider};
use tokio::sync::RwLock;
use trafficstar_errors::traffic_star_error::TrafficStarError;


pub type ResolverType = Resolver<hickory_resolver::name_server::GenericConnector<hickory_resolver::proto::runtime::TokioRuntimeProvider>>;
#[derive(Clone)]
pub struct DnsResolver{
    resolver : Arc<RwLock<ResolverType>>,
    ipv4_map : Arc<RwLock<HashMap<String,Ipv4Addr>>>
}

impl DnsResolver{
    pub fn new(config : ResolverConfig) -> Self{
        let resolver: ResolverType = Resolver::builder_with_config(config,     TokioConnectionProvider::default()).build();
        Self { 
            resolver: Arc::new(RwLock::new(resolver)),
            ipv4_map : Arc::new(RwLock::new(HashMap::new()))
        }
        
    }

    async fn resolv_from_cache(&self, name : &str) -> Option<Ipv4Addr>{
        let lock = self.ipv4_map.read().await;
        lock.get(name).copied()
    }

    async fn resolv_from_resolver(&self, name : &str) -> Result<Ipv4Addr, TrafficStarError>{
        let resolver = self.resolver.write().await;
        let result = match resolver.lookup_ip(name).await{
            Ok(v) => v,
            Err(err) => return Err(format!("Resolver failed, error returned : {}",err).into()),
        };
        let iter = result.iter();
        for entry in iter {
            match entry{
                std::net::IpAddr::V4(ipv4_addr) => return Ok(ipv4_addr),
                std::net::IpAddr::V6(_) => {},
            };
        }

        Err(TrafficStarError::msg("No ipv4 address found!".into()))
    }

    pub async fn resolve_ipv4(&self, name : &str) -> Result<Ipv4Addr, TrafficStarError>{
        if let Some(ipv4) = self.resolv_from_cache(name).await{
            return Ok(ipv4)
        }
        let mut lock = self.ipv4_map.write().await;
        let res = self.resolv_from_resolver(name).await?;
        lock.insert(name.to_string(), res);
        Ok(res)

    }
}