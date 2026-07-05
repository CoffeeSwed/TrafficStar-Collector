use std::{net::Ipv4Addr, str::FromStr};

use trafficstar_logger::{serror};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::trafficstar_networking::interface::TrafficStarInterfaceName;

#[derive(StructLoggerName)]
pub struct IPTableRule{
    pub fwmarks : Vec<u32>,
    pub ips : Vec<Ipv4Addr>,
    pub interfaces : Vec<TrafficStarInterfaceName>,
    pub rule : String,
    pub chain : String,
}

impl IPTableRule{
    ///Input is Rule and then Chain
    pub fn new_vec(rules : Vec<(String,String)>) -> Vec<Self>{
        let mut res = Vec::new();
        for (rule, chain) in rules{
            let interfaces = Self::get_interfaces(&rule);
            let ips = Self::get_ips(&rule);
            let marks = Self::get_fwmark(&rule);
            res.push(Self{
                chain,
                rule,
                fwmarks : marks,
                interfaces,
                ips
            });
        }
        res
    }

    pub fn get_interfaces(rule : &str) -> Vec<TrafficStarInterfaceName>{
        let mut res = Vec::new();
        let mut next_interface = false;
        for part in rule.split(' '){
            if part.eq_ignore_ascii_case("-i") 
            || part.eq_ignore_ascii_case("-o"){

                next_interface = true;
            }else {
                if next_interface && let Ok(name) = TrafficStarInterfaceName::from_str(part){
                        res.push(name);
                }
                next_interface = false;
            }
        }
        
        res
    }

    pub fn get_ips(rule : &str) -> Vec<Ipv4Addr>{
        let mut res = Vec::new();
        let mut next_ip = false;
        for part in rule.split(' '){
            if part.eq_ignore_ascii_case("-s") 
            || part.eq_ignore_ascii_case("-d"){

                next_ip = true;
            }else {
                if next_ip 
                && let Some(ipv4_addr) = part.split('/').next()
                && let Ok(ipv4_addr) = Ipv4Addr::from_str(ipv4_addr){
                    res.push(ipv4_addr);
                }
                next_ip = false;
            }
        }
        
        res
    }

    pub fn get_fwmark(rule : &str) -> Vec<u32>{
        let mut res = Vec::new();
        let mut next_fwmark = false;
        for part in rule.split(' '){
            if part.starts_with("--") 
            && part.ends_with("mark"){
                next_fwmark = true;
            }else {
                if next_fwmark && let Some(mark) = part.split('/').next(){
                    let mark = mark.trim_start_matches("0x");
                    match u32::from_str_radix(mark, 16){
                        Ok(v) => res.push(v),
                        Err(err) => serror!("Invalid mark found {{{}}}, error : {}",mark,err),
                    };
                }
                next_fwmark = false;
            }
        }
        
        res
    }
}