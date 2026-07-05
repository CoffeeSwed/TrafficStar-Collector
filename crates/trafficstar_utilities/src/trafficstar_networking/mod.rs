use std::{ffi::{CStr}, mem::MaybeUninit, net::Ipv4Addr, os::fd::RawFd};

use nix::libc::{self};
use trafficstar_errors::traffic_star_error::TrafficStarError as TrafficStarError;

use crate::trafficstar_networking::interface::{TrafficStarInterfaceName};
mod networking_tests;
pub mod interface;

#[allow(unsafe_code,unused,clippy::needless_return)]
pub fn set_ipv4(interface_name : &TrafficStarInterfaceName, ip: Ipv4Addr, subnet_mask : Option<Ipv4Addr>) -> Result<(),TrafficStarError> {
    #[cfg(target_os = "linux")]{

        /*SAFETY: This blocks set the ip of an interface, using libc! Relies on transmute between sockaddr_in and
        */
        unsafe{
            let fd : RawFd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if fd < 0{
                return Err(std::io::Error::last_os_error().into())
            }

            let mut interface = MaybeUninit::<libc::ifreq>::zeroed();
            let mut interface = interface.assume_init();
            interface.ifr_name = interface_name.name;

            interface.ifr_ifru.ifru_addr.sa_family = libc::AF_INET as u16;
            for (index, byte) in ip.octets().iter().enumerate(){
                //sockaddr is sockaddr_in, so skip first two bytes (the port number)!
                interface.ifr_ifru.ifru_addr.sa_data[index+2] = *byte as i8; //
            }


            // 4. Call ioctl to set the IP
            if libc::ioctl(fd, libc::SIOCSIFADDR, &interface) < 0 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(err.into());
            }
            let mut mask = subnet_mask.unwrap_or(<Ipv4Addr as std::str::FromStr>::from_str("255.255.255.255").unwrap());

            for (index, byte) in mask.octets().iter().enumerate(){
            //sockaddr is sockaddr_in, so skip first two bytes (the port number)!
                interface.ifr_ifru.ifru_addr.sa_data[index+2] = *byte as i8;
            }

            // 4. Call ioctl to set the Netmask
            if libc::ioctl(fd, libc::SIOCSIFNETMASK, &interface) < 0 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(err.into());
            }

            libc::close(fd);
            
        }
        return Ok(())
    }
    return Err(TrafficStarError::id_msg("UnsupportedOS".into(), "Operating system not supported!".into()))

}

#[allow(unsafe_code,unused,clippy::needless_return)]
pub fn set_interface_flag(interface_name : &TrafficStarInterfaceName, flag : libc::c_int) -> Result<(),TrafficStarError> {
    #[cfg(target_os = "linux")]
    {
    /*SAFETY: This blocks set the interface up flag for a interface, using libc!
    */
    unsafe{

        
        let fd : RawFd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
        if fd < 0{
            return Err(std::io::Error::last_os_error().into())
        }
        let mut interface : libc::ifreq = std::mem::zeroed();
        interface.ifr_name = interface_name.name;
        if libc::ioctl(fd, libc::SIOCGIFFLAGS,(&mut interface) as *mut libc::ifreq) < 0{
            libc::close(fd);
            return Err(std::io::Error::last_os_error().into())
        }

        // Set flag
        interface.ifr_ifru.ifru_flags |= flag as i16;

        if libc::ioctl(fd, libc::SIOCSIFFLAGS, (&mut interface) as *mut libc::ifreq) < 0 {
            libc::close(fd);
            return Err(std::io::Error::last_os_error().into());
        }
        
        libc::close(fd);


        }
        return Ok(())
    }
    return Err(TrafficStarError::id_msg("UnsupportedOS".into(), "Operating system not supported!".into()))
}

#[allow(unused,clippy::needless_return,unsafe_code)]
pub fn drop_tunnel(interface_name : &TrafficStarInterfaceName) -> Result<(), TrafficStarError>{
    #[cfg(target_os = "linux")]
    {
    /*SAFETY: This blocks set the interface as none persisent using libc!
    */
    unsafe{
            let fd   = libc::open(c"/dev/net/tun".as_ptr(), libc::O_RDWR);
            if fd < 0{
                return Err(std::io::Error::last_os_error().into())
            }
            let mut interface : libc::ifreq = std::mem::zeroed();
        
            interface.ifr_name = interface_name.name;


            // Bind interface to this fd.
            interface.ifr_ifru.ifru_flags = libc::IFF_TUN as i16| libc::IFF_NO_PI as i16;

            if libc::ioctl(fd, libc::TUNSETIFF, (&mut interface) as *mut libc::ifreq) < 0{
                libc::close(fd);
                return Err(std::io::Error::last_os_error().into());
            }
            
            if libc::ioctl(fd, libc::TUNSETPERSIST, 0 as libc::c_int) < 0 {
                libc::close(fd);
                return Err(std::io::Error::last_os_error().into());
            }


            libc::close(fd);
            return Ok(())
        }
    }
    return Err(TrafficStarError::id_msg("UnsupportedOS".into(), "Operating system not supported!".into()))

}

///Sets it to persistent!
#[allow(unused,clippy::needless_return,unsafe_code)]
pub fn create_tunnel(interface_name : &TrafficStarInterfaceName, ipv4_addr : Option<Ipv4Addr>, subnet_mask : Option<Ipv4Addr>) -> Result<(), TrafficStarError>{
        #[cfg(target_os = "linux")]
    {
    /*SAFETY: This blocks set the interface up flag for a interface, using libc!
    */
    unsafe{
        use std::{fs::OpenOptions, os::fd::{AsRawFd, IntoRawFd, OwnedFd}};
        use log::debug;
        
        




      
        let fd   = libc::open(c"/dev/net/tun".as_ptr(), libc::O_RDWR);
        if fd < 0{
            return Err(std::io::Error::last_os_error().into());
        }
        let mut interface : libc::ifreq = std::mem::zeroed();
       
        interface.ifr_name = interface_name.name;
        

        // Set flags
        interface.ifr_ifru.ifru_flags = libc::IFF_TUN as i16| libc::IFF_NO_PI as i16;
        
        if libc::ioctl(fd.as_raw_fd(), libc::TUNSETIFF, (&mut interface) as *mut libc::ifreq) < 0 {
            libc::close(fd);
            return Err(std::io::Error::last_os_error().into());
        }


        if libc::ioctl(fd, libc::TUNSETPERSIST, 1 as libc::c_int) < 0 {
            libc::close(fd);
            return Err(std::io::Error::last_os_error().into());
        }
        libc::close(fd);

        if let Some(ip) = ipv4_addr 
        && let Err(err ) = set_ipv4(interface_name, ip, subnet_mask)
            {
                return Err(err)
            }
        
        set_interface_flag(interface_name, libc::IFF_UP)?;
        return Ok(())
        }
        
    }
    
    return Err(TrafficStarError::id_msg("UnsupportedOS".into(), "Operating system not supported!".into()))
}


struct IfAddrs(*mut libc::ifaddrs);
impl Drop for IfAddrs {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        /*SAFETY: Frees with the help libc::freeifaddrs! */
        unsafe {
            if !self.0.is_null() {
                libc::freeifaddrs(self.0);
            }
        }
    }
}

#[allow(dead_code, unsafe_code, unreachable_code,clippy::needless_return)]
pub fn get_interfaces() -> Result<Vec<TrafficStarInterfaceName>, TrafficStarError>{
    #[cfg(target_os = "linux")]
    {
    /*SAFETY: Returns interfaces present in current network space, uses libc! */
    unsafe{
            let mut interfacepointer: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut interfacepointer) < 0{
                return Err(std::io::Error::last_os_error().into())
            }
            let mut res: Vec<TrafficStarInterfaceName> = Vec::new();
            let holder = IfAddrs(interfacepointer);
            let mut pointer = holder.0;
            while !pointer.is_null(){
                let name = CStr::from_ptr((*pointer).ifa_name).to_string_lossy().into_owned();
                if res.iter().find(|&x| *x.as_string() == name).is_none(){
                    use std::str::FromStr;

                    res.push(TrafficStarInterfaceName::from_str(&name).unwrap()); 
                }
                pointer = (*pointer).ifa_next;
            }
            drop(holder);
            return Ok(res)
        }
    }
    return Err(TrafficStarError::id_msg("UnsupportedOS".into(), "Operating system not supported!".into()))
}



#[cfg(test)]
#[allow(unused)]
mod tests {
    use std::{ io::{PipeWriter, Write}, os::fd::{AsRawFd, OwnedFd, RawFd}, sync::{Arc, Once}, time::Duration};


    use futures::{AsyncReadExt, AsyncWriteExt};
    use log::{debug, info};
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::{panicerror, trafficstar_logger::TrafficStarLogger};

    use crate::{get_multi_runtime, fetch_public_ip, trafficstar_files::file_handler::FileHandler, trafficstar_networking::get_interfaces};


    #[warn(unused_unsafe)]
    pub fn setup(test_name : String) {
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res {
                log::set_max_level(log::LevelFilter::Debug);
            }
        });
        TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name.clone()] }));
        TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick { nicks: vec![test_name,"CHILD".into()] }));
    }
 
    #[test]
    fn interface_get_test() {
        setup("InterFaceGetTest".into());
        let res = std::thread::spawn(move || {

        let interfaces = get_interfaces()?;
        for (i, interface) in interfaces.iter().enumerate(){
            info!("{} : {}",i,interface.as_string())
        }

        std::result::Result::<(),TrafficStarError>::Ok(())
        }).join().unwrap();
        if let Err(err) = res{
            panicerror!("{}",err)
        }
        
    }
}