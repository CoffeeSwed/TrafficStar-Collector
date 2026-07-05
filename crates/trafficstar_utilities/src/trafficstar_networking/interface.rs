
use std::{fmt::Display, net::Ipv4Addr, str::FromStr};

use nix::libc::{IFNAMSIZ, c_char};
use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::serror;
use trafficstar_logger_macro::StructLoggerName;

use crate::trafficstar_networking;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrafficStarInterfaceName{
    pub name : [c_char; IFNAMSIZ]
}

impl Default for TrafficStarInterfaceName{
    #[allow(unsafe_code)]
    fn default() -> Self {
        // SAFETY: Fastest way to do this, valid string if its all zeros!
        unsafe{
            std::mem::zeroed()
        }
    }
}

impl TrafficStarInterfaceName{
    #[allow(unsafe_code)]
     pub fn as_string(&self) -> &str {
        std::str::from_utf8(self.as_array()).unwrap_or("")
    }

    #[allow(unsafe_code)]
    pub fn as_array(&self) -> &[u8]{
        let bytes = &self.name;

        // Find null terminator
        let len = bytes.iter()
            .position(|&c| c == 0)
            .unwrap_or(bytes.len());
         let slice = &bytes[..len];
        /*SAFETY: We know it contains a null character, hence it's safe! */
        unsafe {
            std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len())
        }
        
    }
}

impl FromStr for TrafficStarInterfaceName{
    type Err = TrafficStarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < IFNAMSIZ{
            let mut res = TrafficStarInterfaceName::default();
            for (index,byte) in s.as_bytes().iter().enumerate(){
                res.name[index] = *byte as c_char;
            }
            Ok(res)

        }else{
            Err(std::io::Error::new(std::io::ErrorKind::InvalidFilename, "Filename is to large!").into())
        }
    }
}

impl Display for TrafficStarInterfaceName{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.as_string())
    }
}

#[derive(StructLoggerName)]
pub struct TrafficStarInterface{
    pub name : TrafficStarInterfaceName,
}

impl TrafficStarInterface{
    pub fn new(name : &str, ipv4_addr : Option<Ipv4Addr>, ipv4_mask : Option<Ipv4Addr>) -> Result<Self, TrafficStarError>{
        let res = Self{
            name : TrafficStarInterfaceName::from_str(name)?
        };
        trafficstar_networking::create_tunnel(&res.name, ipv4_addr, ipv4_mask)?;
        Ok(res)
    }
}

impl Drop for TrafficStarInterface{
    fn drop(&mut self) {
        let name = self.name;
        while let Err(err) = trafficstar_networking::drop_tunnel(&name){
            if let Some(io) = err.get_ioerror() 
            && io.kind() == std::io::ErrorKind::ResourceBusy
            {
                continue;
            }
            
            serror!("Could not delete interface, reason is : {}",err);
        }
    }
}