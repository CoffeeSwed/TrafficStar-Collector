use std::{net::Ipv4Addr, str::FromStr, sync::Arc};

use futures::{StreamExt, TryStreamExt};
use iptables::IPTables;
use rtnetlink::{RouteMessageBuilder, packet_core::NetlinkMessage, packet_route::{RouteNetlinkMessage, link::LinkAttribute, route::RouteMessage, rule::RuleMessage}};
use tokio::{sync::RwLock, task::JoinHandle};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{trafficstar_logger::TrafficStarLogger};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{async_run_command, trafficstar_networking::interface::TrafficStarInterfaceName};

use crate::iptable_rule::IPTableRule;


#[derive(StructLoggerName)]
pub struct InterfaceController{
    rt_task : Arc<JoinHandle<()>>,
    rt_handle : Arc<RwLock<rtnetlink::Handle>>,
    _rt_inner : Arc<futures::channel::mpsc::UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, rtnetlink::sys::SocketAddr)>>,
    wg_client : Arc<RwLock<wgctrl::client::Client>>,
    iptables : Arc<RwLock<IPTables>>
}

impl InterfaceController{
    pub fn new() -> Result<Self, TrafficStarError>{
        TrafficStarLogger::mute("netlink_packet_route::link::buffer_tool".into(), log::Level::Warn);
        let wg_client = match wgctrl::client::Client::new(){
            Ok(v) => Arc::new(RwLock::new(v)),
            Err(err) => return Err(format!("Failed creating wg_client handler, {}",err).into()),
        };
        let (connection_rt, handle_rt, rt_inner) = match rtnetlink::new_connection() {
            Ok(v) => v,
            Err(err) => return Err(format!("Failed creating rtnetlink handler, err {}",err).into()),
        };
        let iptable = match iptables::new(false){
            Ok(v) => v,
            Err(err) => return Err(format!("Failed creating iptables handler, err {}",err).into()),
        };
        Ok(
            Self{
                rt_task : Arc::new(tokio::spawn(connection_rt)),
                _rt_inner : Arc::new(rt_inner),
                rt_handle : Arc::new(RwLock::new(handle_rt)),
                wg_client,
                iptables : Arc::new(RwLock::new(iptable))
            }
        )
    }

    pub async fn get_links(&self) -> Result<Vec<TrafficStarInterfaceName>, TrafficStarError>{
        let handle = self.rt_handle.write().await;
        let mut res = Vec::new();
        let mut links = handle.link().get().execute();
        loop{
            match links.try_next().await{
                Ok(v) => {
                    if let Some(link) = v{
                        for nla in link.attributes.into_iter() {
                            if let LinkAttribute::IfName(name) = nla {
                                res.push(TrafficStarInterfaceName::from_str(&name)?);
                                break;
                            }
                        }
                    }else{
                        break;
                    }
                },
                Err(err) => {
                    return Err(format!("Communication error : {}",err).into())
                },
            }
        }
        
        Ok(res)
    }


    pub async fn delete_link(&self, name : &TrafficStarInterfaceName) -> Result<(), TrafficStarError>{
        let handle = self.rt_handle.write().await;
        let mut links = handle.link().get().match_name(name.as_string().to_string()).execute();
        match links.try_next().await{
            Ok(v) => {
                if let Some(link) = v{
                    if let Err(err) = handle.link().del(link.header.index).execute().await{
                        return Err(format!("Got error when deleting interface {}, err : {}!",name.as_string(), err).into())
                    }
                }else{
                    return Err(format!("Could not find interface {}!",name.as_string()).into())
                }
            },
            Err(err) => {
                return Err(format!("Communication error : {}",err).into())
            },
        }
        
        Ok(())        
    }


    pub async fn get_all_ipv4_addresses(&self) -> Result<Vec<Ipv4Addr>, TrafficStarError>{
        let mut res = Vec::new();
        for link in self.get_links().await?{
            res.append(&mut self.get_ipv4_addresses(&link).await?);
        }
        Ok(res)
    }

    pub async fn get_ipv4_addresses(&self, interface : &TrafficStarInterfaceName) -> Result<Vec<Ipv4Addr>, TrafficStarError>{
        let handle = self.rt_handle.write().await;
        let mut res_vec = Vec::new();
        let link = match handle.link().get().match_name(interface.to_string()).execute().try_next().await{
            Ok(v) => {
                if let Some(v) = v {
                    v
                }else{
                    return Err("Failed to find interface, none had a matching name!".into())
                }
            },
            Err(err) => return Err(format!("Received error to find interface, error : {}",err).into()),
        };
        let mut address = handle.address().get().set_link_index_filter(link.header.index).execute();
        loop{
            let res = match address.try_next().await{
                Ok(v) => v,
                Err(err) => {
                    return Err(format!("Failed getting ip address of interface, error : {}",err).into())
                },
            };
            if let Some(res) = res{
                for attr in res.attributes{
                    match attr{
                        rtnetlink::packet_route::address::AddressAttribute::Address(ip_addr) => {
                            if ip_addr.is_ipv4(){
                                let address = ip_addr.as_octets();
                                res_vec.push(Ipv4Addr::new(address[0], address[1], address[2], address[3]));
                            }
                        },
                        _ => continue,
                    };
                }
            }else{
                break;
            }
        }

        Ok(res_vec)
    }


    pub async fn get_used_fwmarks(&self) -> Result<Vec<u32>, TrafficStarError>{
        let mut res = Vec::new();
        let handle = self.rt_handle.write().await;

        let mut routes = handle.rule().get(rtnetlink::IpVersion::V4).execute();
        loop{
            match routes.try_next().await{
                Ok(route) => {
                    if let Some(route) = route{
                        for attributes in route.attributes{
                            if let rtnetlink::packet_route::rule::RuleAttribute::FwMark(v) = attributes {
                                if !res.contains(&v){
                                    res.push(v);
                                }
                                break;
                            };
                        }
                    }else{
                        break;
                    }
                },
                Err(err) => return Err(format!("{}",err).into()),
            }
        }
        
        let mut wg_client = self.wg_client.write().await;
        for device in match wg_client.list_devices(){
            Ok(v) => v,
            Err(err) => return Err(format!("Failed getting wireguard interfaces, err {}",err).into()),
        }{
             if device.firewall_mark != 0
                && !res.contains(&device.firewall_mark){
                    
                res.push(device.firewall_mark);
             }
        }
        drop(handle);

        for table in self.get_iptables(){
            for rules in IPTableRule::new_vec(self.get_rules_iptable(&table).await?){
                for fwmark in rules.fwmarks{
                    if !res.contains(&fwmark){
                        res.push(fwmark);
                    }
                }
            }
        }

        for routem in self.get_routes().await?{
            for attribute in &routem.attributes{
                if let rtnetlink::packet_route::route::RouteAttribute::Mark(v) = attribute 
                && !res.contains(v){
                    res.push(*v);
                }
            }
        }
        Ok(res)
    }

    
    pub async fn get_rules_ip(&self) -> Result<Vec<RuleMessage>, TrafficStarError>{
        let handle = self.rt_handle.write().await;
        let mut rules = handle.rule().get(rtnetlink::IpVersion::V4).execute();
        let mut rules_res = Vec::new();
        loop{
            let rule_fetched = match rules.try_next().await{
                Ok(v) => v,
                Err(err) => return Err(format!("Unable to fetch rules, error : {}",err).into()),
            };
            if let Some(rule) = rule_fetched{
                rules_res.push(rule);
            }else{
                break;
            }
        }
        Ok(rules_res)
    }

    pub async fn get_tables(&self) -> Result<Vec<u32>, TrafficStarError>{
        let mut res : Vec<u32> = Vec::new();
        for rule in self.get_rules_ip().await?{
            for attribute in rule.attributes{
                if let rtnetlink::packet_route::rule::RuleAttribute::Table(v) = attribute 
                && !res.contains(&v){
                    res.push(v);
                }
                if let rtnetlink::packet_route::rule::RuleAttribute::Goto(v) = attribute 
                && !res.contains(&v){
                    res.push(v);
                }
            }
        }      

        for route in self.get_routes().await?{
            for attribute in route.attributes{
                if let rtnetlink::packet_route::route::RouteAttribute::Table(v) = attribute 
                && !res.contains(&v){
                    res.push(v);
                }
            }
        }      
        

        Ok(res)
    }

    pub async fn delete_rule(&self, rule : RuleMessage) -> Result<(),TrafficStarError>{
        let handler = self.rt_handle.write().await;
        if let Err(err) = handler.rule().del(rule).execute().await{
            Err(format!("Could not delete rule, error : {}",err).into())
        }else{
            Ok(())
        }
    }

    pub async fn get_chains_iptable(&self, table : &str) -> Result<Vec<String>, TrafficStarError>{
        let iptables = self.iptables.write().await;
        
        match iptables.list_chains(table){
            Ok(v) => Ok(v),
            Err(err) => {
                Err(format!("Error received : {}",err).into())
            },
        }
    }

    ///Returns (rule, chain)
    pub async fn get_rules_iptable(&self, table : &str) -> Result<Vec<(String, String)>, TrafficStarError>{
        let iptables = self.iptables.write().await;
        let rules = match iptables.list_table(table){
            Ok(v) => v,
            Err(err) => {
                
                return Err(format!("Error received : {}",err).into())
            },
        };

        let mut res = Vec::new();
        for line in rules{
            let splits : Vec<&str> = line.splitn(3," ").collect();
            if splits.len() > 2{
                res.push((splits[2].to_string(),splits[1].to_string()));
            }else{
                res.push((splits[1].to_string(),splits[0].to_string()));
            }
        }
        Ok(res)
    }


    pub fn get_iptables(&self) -> Vec<String>{
        vec!["filter".into(),"nat".into(),"mangle".into(),"raw".into(),"security".into()]
    }

    pub async fn delete_iptables_rule(&self, table : String, chain : String, rule : String) -> Result<(),TrafficStarError>{
        
        let iptable = self.iptables.write().await;
        let mut command = vec!["-t",&table,"-D",&chain, "-w"];
        for part in rule.split_whitespace(){
            command.push(part);
        }
        let _ = async_run_command("iptables", command).await;
        drop(iptable);
        Ok(())
    }

    pub async fn get_routes(&self) -> Result<Vec<RouteMessage>,TrafficStarError>{
        let handle = self.rt_handle.write().await;
        let routemessage = RouteMessageBuilder::<Ipv4Addr>::new().build();
        let mut res = handle.route().get(routemessage).execute();
        let mut res_vector = Vec::new();
        while let Some(entry) = res.next().await{
            match entry {
                Ok(v) => res_vector.push(v),
                Err(err) => return Err(format!("Received unexpected error retriving routes, error : {}",err).into()),
            }
        }
        
        Ok(res_vector)
    }

    pub async fn del_route(&self, route : RouteMessage) -> Result<(),TrafficStarError>{
        let handle = self.rt_handle.write().await;
        if let Err(err) = handle.route().del(route).execute().await{
            Err(format!("received unexecpted error deleting route, error : {}",err).into())
        }else{
            Ok(())
        }
    }
    
}

impl Drop for InterfaceController{
    fn drop(&mut self) {
        self.rt_task.abort();
    }
}