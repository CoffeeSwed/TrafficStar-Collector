use std::{fs::{self}, mem::MaybeUninit, net::Ipv4Addr, path::PathBuf, str::FromStr, sync::{Arc, Mutex, Once}};

use platform_dirs::AppDirs;
use rand::random;
use rtnetlink::packet_route::route::RouteAddress;
use tokio::runtime::Runtime;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{panicerror, serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{get_multi_runtime, run_command, trafficstar_networking::interface::TrafficStarInterfaceName};

use crate::{LinkType, cached_vec::CachedVec, controller::InterfaceController, iptable_rule::IPTableRule};

#[allow(unused)]
#[derive(StructLoggerName, Clone)]
pub struct ReservationController{
    used_tables : Arc<Mutex<CachedVec<u32>>>,
    used_names : Arc<Mutex<CachedVec<TrafficStarInterfaceName>>>,
    used_ips : Arc<Mutex<CachedVec<Ipv4Addr>>>,
    used_fwmarks : Arc<Mutex<CachedVec<u32>>>,
    controller : Arc<InterfaceController>,
    rt : Arc<Runtime>,
}

impl ReservationController{

    #[allow(unsafe_code, static_mut_refs)]
    ///Static behaviour required
    pub fn instance() -> Arc<ReservationController> {
        static mut SINGLETON: MaybeUninit<Arc<ReservationController>> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        // SAFETY:
        // Needed to create a singleton of an initializable as a static.
        unsafe {
            ONCE.call_once(|| {

                let used_names = Arc::new(Mutex::new(Vec::new()));
                let used_ips = Arc::new(Mutex::new(Vec::new()));
                let rt = match get_multi_runtime(){
                    Ok(v) => v,
                    Err(err) => {
                        panicerror!("Could not create needed runtime, error : {}",err);
                    }
                };
                let controller = match rt.block_on(async move {InterfaceController::new()}){
                    Ok(v) => Arc::new(v),
                    Err(err) => panicerror!("Could not create needed Interface Controller, error : {}",err),
                };
                
                
                if let Ok(interfaces) = rt.block_on(controller.get_links())
                {
                    let mut used_names = used_names.lock().unwrap();
                    let mut used_ips = used_ips.lock().unwrap();
                    for interface in interfaces{
                        used_names.push(interface);
                        if let Ok(mut ips) = rt.block_on(controller.get_ipv4_addresses(&interface)){
                            used_ips.append(&mut ips);
                        }
                    }    
                    
                }
                    


                    
                

                let appdir = AppDirs::new(Some("trafficstar_interfaces"), false).unwrap().cache_dir;
                if !appdir.exists() && 
                let Err(err) = fs::create_dir(&appdir){
                    panicerror!("Could not create needed state_dir {}, error : {}!",appdir.as_path().to_str().unwrap(),err)
                }
                let cached_interfaces = 
                match CachedVec::<TrafficStarInterfaceName>::new(appdir.clone().join(PathBuf::from_str("/interface.cache").unwrap())){
                    Ok(v) => v,
                    Err(err) => panicerror!("{}",err),
                };
                let cached_ips = match CachedVec::<Ipv4Addr>::new(
                    appdir.clone().join(PathBuf::from_str("/ipv4_address.cache").unwrap())){
                    Ok(v) => v,
                    Err(err) => panicerror!("{}",err),
                };
                let cached_tables = match CachedVec::<u32>::new(
                    appdir.clone().join(PathBuf::from_str("/cached_table.cache").unwrap())){
                    Ok(v) => v,
                    Err(err) => panicerror!("{}",err),
                };
                let cached_fwmarks = match CachedVec::<u32>::new(
                    appdir.clone().join(PathBuf::from_str("/cached_fwmark.cache").unwrap())){
                    Ok(v) => v,
                    Err(err) => panicerror!("{}",err),
                };
                let single = ReservationController{
                    rt,
                    controller,
                    used_tables : Arc::new(Mutex::new(cached_tables)),
                    used_names : Arc::new(Mutex::new(cached_interfaces)),
                    used_ips : Arc::new(Mutex::new(cached_ips)),
                    used_fwmarks : Arc::new(Mutex::new(cached_fwmarks)),
                };
                let _ = single.delete_old_entries();
                
                SINGLETON.write(Arc::new(single));
                
                
            });

            SINGLETON.assume_init_mut().clone()
        }
    }


    fn delete_old_entries(&self) -> Result<(),TrafficStarError>{
        let used_interfaces = self.used_names.lock().unwrap().clone();
        let used_tables = self.used_tables.lock().unwrap().clone();
        let used_ips = self.used_ips.lock().unwrap().clone();
        let used_fwmarks = self.used_fwmarks.lock().unwrap().clone();



        //interfaces
        for interface in &*used_interfaces{
            self.free_interface(*interface);
        }

        //tables
        for rule in &*used_tables{
            self.free_table(*rule);
        }

        //ips
        for address in &*used_ips{
            self.free_ipaddress(address);
        }

        //
        for fwmark in &*used_fwmarks{
            self.free_fwmark(*fwmark);
        }

        Ok(())
        
        
    }

    pub fn reserv_mark(&self) -> u32{
        let mut lock = self.used_fwmarks.lock().unwrap();
        let used = self.rt.block_on(async move {
            self.controller.get_used_fwmarks().await.unwrap_or(Vec::new())
        });
        
        let mut found = 300;
        while found < u32::MAX{
            if !lock.contains(&found) && !used.contains(&found){
                lock.push(found);
                let _ = lock.write_cache();
                return found;
            }

            found += 1;
        }
        panicerror!("ALL FWMARKS ARE USED!");
    }

    pub fn free_fwmark(&self, fwmark : u32){
        let mut lock = self.used_fwmarks.lock().unwrap();
        let index = lock.iter().position(|x| *x == fwmark);
        if let Some(index) = index{
            self.rt.block_on(async move{
            if let Ok(rules) = self.controller.get_rules_ip().await{
                for rule in rules{
                    for attributes in &rule.attributes{
                        match attributes{
                            
                            rtnetlink::packet_route::rule::RuleAttribute::FwMark(val) => {
                                if *val == fwmark{
                                    let _ = self.controller.delete_rule(rule).await;
                                    break;
                                }
                            },
                            _ => {
                                continue;
                            },
                        };
                    }
                }
            }
            if let Ok(routes) = self.controller.get_routes().await{
                for route in &routes{
                    for attribute in &route.attributes{
                        match attribute{
                            
                            rtnetlink::packet_route::route::RouteAttribute::Mark(val) => {
                                if *val == fwmark{
                                    let _ = self.controller.del_route(route.clone()).await;
                                    break;
                                }
                            },
                            _ => {
                                continue;
                            },
                        };
                    }
                }
            }
            for table in self.controller.get_iptables(){
                for entry in IPTableRule::new_vec(self.controller.get_rules_iptable(&table).await.unwrap()){
                    if entry.fwmarks.contains(&fwmark){
                        let _ = self.controller.delete_iptables_rule(table.clone(), entry.chain, entry.rule).await;
                    }
                }
            }
            });
            lock.remove(index);
            let _ = lock.write_cache();
        }
    }

    ///Will be deleted automatically if exists afterwards!
    pub fn create_and_reserv_random_interface_name(&self, start_name : Option<String>) -> TrafficStarInterfaceName{
        const MAX_INTERFACE_NAME : usize = 15;
        const ALLOWED_CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let mut lock = self.used_names.lock().unwrap();
        let declared_interfaces = self.rt.block_on(self.controller.get_links());
        loop{
            let mut name = start_name.clone().unwrap_or("ts".to_string());
            while name.len() < MAX_INTERFACE_NAME{
                let character_num = random::<usize>() % ALLOWED_CHARACTERS.len();
                let character = ALLOWED_CHARACTERS.chars().nth(character_num).unwrap();
                name.push(character);
            }
            let name = TrafficStarInterfaceName::from_str(&name).unwrap();
            if !lock.contains(&name){
                if let Ok(used_names) = &declared_interfaces 
                && used_names.contains(&name){
                    continue;
                }
                
                lock.push(name);
                if let Err(err) = lock.write_cache(){
                    panicerror!("COULD NOT WRITE DOWN, ERROR IS : {}",err);
                }
                let _ = lock.write_cache();
                return name;
            }
        }
    }


    pub fn free_interface(&self, name : TrafficStarInterfaceName){
        let mut used_names = self.used_names.lock().unwrap();
        let index = used_names.iter().position(|x| *x == name);
        if let Some(index) = index{
            self.rt.block_on(async move{
            if let Ok(rules) = self.controller.get_rules_ip().await{
                for rule in rules{
                    for attributes in &rule.attributes{
                        match attributes{
                            
                            rtnetlink::packet_route::rule::RuleAttribute::Iifname(val) => {
                                if *val == name.as_string(){
                                    let _ = self.controller.delete_rule(rule).await;
                                    break;
                                }
                            },
                            
                            rtnetlink::packet_route::rule::RuleAttribute::Oifname(val) => {
                                if *val == name.as_string(){
                                    let _ = self.controller.delete_rule(rule).await;
                                    break;
                                }
                            },
                            _ => {
                                continue;
                            },
                        };
                    }
                }
            }
            for table in self.controller.get_iptables(){
                for (rule,chain) in self.controller.get_rules_iptable(&table).await.unwrap(){
                    if rule.contains(name.as_string()){
                        let _ = self.controller.delete_iptables_rule(table.clone(), chain, rule).await;
                    }
                }
            }
            if let Err(_err) = self.controller.delete_link(&name).await{
                //serror!("Could not delete old link, error : {}",err);
            }
            });
            used_names.remove(index);
            let _ = used_names.write_cache();
        }
    }

    pub fn reserv_new_ipaddress(&self) -> Ipv4Addr{
        loop{
            let ipv4 = Ipv4Addr::from_octets([10_u8,random::<u8>(),random::<u8>(),random::<u8>()]);
            if self.reserv_ipaddress(&ipv4){
                return ipv4
            }
        }
    }

    ///Returns true if it could reserv the ip, otherwise false!
    pub fn reserv_ipaddress(&self, address : &Ipv4Addr) -> bool{
        let mut used_ips = self.used_ips.lock().unwrap();
        let declared_ipv4s = self.rt.block_on(self.controller.get_all_ipv4_addresses());

        if !used_ips.contains(address){
            if let Ok(found_ips) = &declared_ipv4s 
            && found_ips.contains(address){
                return false;
            }
            used_ips.push(*address);
            let _ = used_ips.write_cache();
            true
        }
        else{
            false
        }
    }


    pub fn free_table(&self, table : u32){
        let mut used_ids = self.used_tables.lock().unwrap();
        if let Some(index) = used_ids.iter().position(|x| *x == table){
            self.rt.block_on(async move{
                    if let Ok(rules) = self.controller.get_rules_ip().await{
                        for rule in rules{
                            if rule.attributes.iter().position(|x| match x{
                            rtnetlink::packet_route::rule::RuleAttribute::Table(value) => {
                                table == *value
                                },
                                _ => false,
                            } 
                            
                        ).is_some()
                                && let Err(_err) = self.controller.delete_rule(rule).await{
                                    //serror!("couldn't delete table, error is : {}",err);
                                }
                        }
                    }

                if let Ok(routes) = self.controller.get_routes().await{
                    for route in routes{
                        for attribute in &route.attributes{
                            match attribute{
                                rtnetlink::packet_route::route::RouteAttribute::Table(v) => {
                                    if table == *v{
                                        let _ = self.controller.del_route(route.clone()).await;
                                        break;
                                    }
                                },
                                _ => continue,
                            };
                        }
                    }
                }
            });

            
            
            used_ids.remove(index);
            let _ = used_ids.write_cache();

        }


    }

    


    pub fn free_ipaddress(&self, address : &Ipv4Addr){
        let mut used_ips = self.used_ips.lock().unwrap();
        if let Some(index) = used_ips.iter().position(|x| *x == *address){
            self.rt.block_on(async move {
                if let Ok(rules) = self.controller.get_rules_ip().await{
                    for rule in rules{
                        for attributes in &rule.attributes{
                            match attributes{
                                rtnetlink::packet_route::rule::RuleAttribute::Destination(ip_addr) => {
                                    if *ip_addr == *address{
                                        let _ = self.controller.delete_rule(rule).await;
                                        break;
                                    }
                                },
                                rtnetlink::packet_route::rule::RuleAttribute::Source(ip_addr) => {
                                    if *ip_addr == *address{
                                        let _ = self.controller.delete_rule(rule).await;
                                        break;
                                    }
                                },
                                rtnetlink::packet_route::rule::RuleAttribute::Oifname(_) => {},
                                _ => continue,
                            };
                        }
                    }
                }

                for table in self.controller.get_iptables(){
                    for (rule,chain) in self.controller.get_rules_iptable(&table).await.unwrap(){
                        if rule.contains(&(address.to_string()+"/32")){
                            let _ = self.controller.delete_iptables_rule(table.clone(), chain, rule).await;
                        }
                    }
                }

                if let Ok(routes) = self.controller.get_routes().await{
                    let address = *address;
                    for route in routes{
                        for attribute in &route.attributes{
                            match attribute{
                                rtnetlink::packet_route::route::RouteAttribute::Destination(ip_addr) => {
                                    if *ip_addr == RouteAddress::from(address){
                                        let _ = self.controller.del_route(route.clone()).await;
                                        break;
                                    }
                                },
                                rtnetlink::packet_route::route::RouteAttribute::Source(ip_addr) => {
                                    if *ip_addr == RouteAddress::from(address){
                                        let _ = self.controller.del_route(route.clone()).await;
                                        break;
                                    }
                                },
                                _ => continue,
                            };
                        }
                    }
                }
                
            });
            
            used_ips.remove(index);
            let _ = used_ips.write_cache();
        }
    }

    pub fn reserv_table(&self) -> u32{
        let mut used = self.used_tables.lock().unwrap();
        let declared_tables = self.rt.block_on(self.controller.get_tables());
        sinfo!("Declared tables : {:?}",declared_tables);
        for id in 300..u32::MAX{
            if !used.contains(&id){
                if let Ok(some) = &declared_tables
                    && some.contains(&id){
                        continue;
                    }
                used.push(id);
                let _ = used.write_cache();
                return id;
            }
        }
        panicerror!("ALL tables are used!");
    }
    
    /*
    Sweet Lord have Mercy
     */
    pub fn create_peer_forwarding_reservation(&self, out_interface : TrafficStarInterfaceName, is_layer_3 : Option<LinkType>) -> Result<Arc<PeerForwardingReservation>, TrafficStarError>{
        let vparent = InterfaceReservation::new(Some("vp-".into()));
        let vchild = InterfaceReservation::new(Some("vc-".into()));
        let vip = Ipv4Reservation::new();
        let cip = Ipv4Reservation::new();
        let mark_reservation = MarkReservation::new();
        let table_id = TableReservation::new();

        run_command("ip", vec!["link","add",vparent.get_name().as_string(),"type","veth","peer","name",vchild.get_name().as_string()])?;
        run_command("ip", vec!["addr","add",&(vip.get_ip().to_string()+"/8"),"dev",vparent.get_name().as_string()])?;
        run_command("ip", vec!["link","set","up",vparent.get_name().as_string()])?;
        //run_command("ip", vec!["link","set","dev",vparent.get_name().as_string(),"arp","off"])?;
        let link_type = is_layer_3.unwrap_or(LinkType::PURE);

        match link_type{
            LinkType::TOR => {
               if let Err(err) = run_command("ip", vec!["route",
                "add","default","dev",out_interface.as_string(),"table",&table_id.get_table().to_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
                if let Err(err) = run_command("ip", vec!["route",
                "add",&(cip.get_ip().to_string()+"/32"),"dev",vparent.get_name().as_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
                if let Err(err) = run_command("ip", vec!["rule",
                "add","fwmark",&mark_reservation.get_mark().to_string(),"lookup",&table_id.get_table().to_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }

                if let Err(err) = run_command("iptables", 
                vec!["-t","mangle",
                "-A","PREROUTING","-i",vparent.get_name().as_string(),
                "-j","MARK",
                "--set-mark",&mark_reservation.get_mark().to_string(),
                "-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }            

                if let Err(err) = run_command("iptables", 
                vec!["-t","nat","-A","POSTROUTING","-s",&(cip.get_ip().to_string()+"/32"),
                "-j","MASQUERADE","-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }

                
                if let Err(err) = run_command("iptables", 
                vec!["-A","FORWARD","-i",vparent.get_name().as_string(),"-o",out_interface.as_string(),
                "-j","ACCEPT","-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
            },
            LinkType::MULLVAD => {

                if let Err(err) = run_command("ip", vec!["route",
                "add","default","dev",out_interface.as_string(),"table",&table_id.get_table().to_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
                if let Err(err) = run_command("ip", vec!["route",
                "add",&(cip.get_ip().to_string()+"/32"),"dev",vparent.get_name().as_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
                if let Err(err) = run_command("ip", vec!["rule",
                "add","fwmark",&mark_reservation.get_mark().to_string(),"lookup",&table_id.get_table().to_string()]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }

                if let Err(err) = run_command("iptables", 
                vec!["-t","mangle",
                "-A","PREROUTING","-i",vparent.get_name().as_string(),
                "-j","MARK",
                "--set-mark",&mark_reservation.get_mark().to_string(),
                "-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }            

                if let Err(err) = run_command("iptables", 
                vec!["-t","nat","-A","POSTROUTING","-s",&(cip.get_ip().to_string()+"/32"),
                "-j","MASQUERADE","-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }

                
                if let Err(err) = run_command("iptables", 
                vec!["-A","FORWARD","-i",vparent.get_name().as_string(),"-o",out_interface.as_string(),
                "-j","ACCEPT","-w"]){
                    serror!("Peer wont be able to forward, error : {}",err);
                }
            },
            LinkType::PURE => {

            },
        };

            //let out_inteface_ip = self.rt.block_on(self.controller.get_ipv4_addresses(&out_interface))?[0];



        
        
        
        
       run_command("ip", vec!["route","flush","cache"])?;
        Ok(
            Arc::new(PeerForwardingReservation{
                vchild_ip : cip,
                vchild_name : vchild,
                vparent_ip : vip,
                vparent_name : vparent,
                vdestination : out_interface,
                table_id,
                mark : mark_reservation
            })
        )
    }
    
    pub fn delete_interface(&self, name : &TrafficStarInterfaceName) -> Result<(),TrafficStarError>{
        self.rt.block_on(self.controller.delete_link(name))
    }
}

#[derive(StructLoggerName)]
pub struct InterfaceReservation{
    name : TrafficStarInterfaceName
}

impl InterfaceReservation{
    pub fn new(start_name : Option<String>) -> Arc<Self>{
        Arc::new(Self{
            name : ReservationController::instance().create_and_reserv_random_interface_name(start_name)
        })
    }

    pub fn get_name(&self) -> TrafficStarInterfaceName{
        self.name
    }
}

impl Drop for InterfaceReservation{
    fn drop(&mut self) {
        let name = self.get_name();
        std::thread::spawn(move || {
                ReservationController::instance().free_interface(name);

        });
    }
}

#[derive(StructLoggerName)]
pub struct Ipv4Reservation{
    ip : Ipv4Addr
}

impl Ipv4Reservation{
    pub fn new() -> Arc<Self> {
        Arc::new(Self { ip: ReservationController::instance().reserv_new_ipaddress() })
    }
}

impl Ipv4Reservation{

    pub fn get_ip(&self) -> Ipv4Addr{
        self.ip
    }
}

impl Drop for Ipv4Reservation{
    fn drop(&mut self) {
        let ip = self.get_ip();
        std::thread::spawn(move || {
                ReservationController::instance().free_ipaddress(&ip);

        });

    }
}

#[derive(StructLoggerName)]
pub struct TableReservation{
    table : u32
}

impl TableReservation{
    pub fn new() -> Arc<Self> {
        Arc::new(Self { table: ReservationController::instance().reserv_table() })
    }
}

impl TableReservation{

    pub fn get_table(&self) -> u32{
        self.table
    }
}

impl Drop for TableReservation{
    fn drop(&mut self) {
        let table = self.get_table();
        std::thread::spawn(move || {
            ReservationController::instance().free_table(table);

        });

    }
}

#[derive(StructLoggerName)]
pub struct MarkReservation{
    mark : u32
}

impl MarkReservation{
    pub fn new() -> Arc<Self> {
        Arc::new(Self { mark: ReservationController::instance().reserv_mark() })
    }
}

impl MarkReservation{

    pub fn get_mark(&self) -> u32{
        self.mark
    }
}

impl Drop for MarkReservation{
    fn drop(&mut self) {
        let mark = self.get_mark();
        std::thread::spawn(move || {
            ReservationController::instance().free_fwmark(mark);

        });

    }
}



pub struct PeerForwardingReservation{
    pub vparent_name : Arc<InterfaceReservation>,
    pub vchild_name : Arc<InterfaceReservation>,
    pub vdestination : TrafficStarInterfaceName,

    pub vparent_ip : Arc<Ipv4Reservation>,
    pub vchild_ip : Arc<Ipv4Reservation>,
    pub table_id : Arc<TableReservation>,
    pub mark : Arc<MarkReservation>
}