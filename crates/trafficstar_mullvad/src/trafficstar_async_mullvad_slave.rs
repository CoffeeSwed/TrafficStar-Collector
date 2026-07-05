use std::{net::Ipv4Addr, str::FromStr, sync::Arc};

use futures::{TryStreamExt, channel::mpsc::UnboundedReceiver};
use rtnetlink::{Handle, LinkUnspec, LinkWireguard, new_connection, packet_core::NetlinkMessage, packet_route::RouteNetlinkMessage};
use tokio::{sync::{RwLock}, task::JoinHandle};
use trafficstar_connections::trafficstar_wireguard::WireguardKeys;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_interface::reservation::ReservationController;
use trafficstar_logger::{panicerror, sdebug, serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{trafficstar_async_queue::AsyncSharedQueue, trafficstar_networking::interface::TrafficStarInterfaceName, trafficstar_pipes::TrafficStarPipePairAsync};

use crate::{trafficstar_async_mullvad_account::AsyncMullvadAccount, trafficstar_async_mullvad_handler_structs::{Command, DeviceDeletionEvents, DeviceEvents, DevicePopperEvents, Errors}, trafficstar_mullvad_device::MullvadDevice};


struct AsyncMullvadSlaveAccountEntryHolder{
    known_devices : Vec<(MullvadDevice,WireguardKeys)>,
}

#[derive(StructLoggerName, Clone)]
struct AsyncMullvadSlaveAccountEntry{
    inner : Arc<AsyncMullvadAccount>,
    devices : Arc<RwLock<AsyncMullvadSlaveAccountEntryHolder>>
}

impl AsyncMullvadSlaveAccountEntry{
    async fn count_known_devices(&self) -> usize{
        let devices = self.devices.read().await;
        
        devices.known_devices.len()
    }

    async fn add_known_device(&self, device : MullvadDevice, keys : WireguardKeys){
        let mut devices = self.devices.write().await;

        devices.known_devices.push((device,keys));
    }

    async fn knows_device(&self, device : &MullvadDevice) -> bool{
        let devices = self.devices.read().await;
        for key in &devices.known_devices{
            if device.pubkey.eq(&key.1.pubkey.to_string()){
                return true;
            }
        }
        false
    }

    async fn forget_device(&self, device : &MullvadDevice){
        let mut devices = self.devices.write().await;
        let mut index : Option<usize> = None;
        for (i,key) in devices.known_devices.iter().enumerate(){
            if key.1.pubkey.to_string() == device.pubkey{
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index{
            devices.known_devices.remove(index);
        }
    }
}


#[allow(unused)]
#[derive(StructLoggerName, Clone)]
pub struct AsyncMullvadSlave{
    command_channel : TrafficStarPipePairAsync,
    accounts : Arc<RwLock<Vec<AsyncMullvadSlaveAccountEntry>>>,
    device_events : Arc<AsyncSharedQueue<DeviceEvents>>,
    device_queue : Arc<AsyncSharedQueue<(MullvadDevice, WireguardKeys)>>,
    device_delete_events : Arc<AsyncSharedQueue<DeviceDeletionEvents>>,
    device_pop_events : Arc<AsyncSharedQueue<DevicePopperEvents>>,
    rt_join_handle : Arc<JoinHandle<()>>,
    rt_handle : Arc<RwLock<Handle>>,
    rt_inner : Arc<UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, rtnetlink::sys::SocketAddr)>>,

}


impl AsyncMullvadSlave{
    const MAX_DEVICES_ACCOUNT : usize = 5;
    
    pub async fn new(command_channel : TrafficStarPipePairAsync) -> Self{
        
        let (connection_rt, handle_rt, rt_inner) = new_connection().unwrap();
         Self { command_channel, 
            accounts : Arc::new(RwLock::new(vec![])),
            device_events : Arc::new(AsyncSharedQueue::new()),
            device_queue : Arc::new(AsyncSharedQueue::new()),
            device_delete_events : Arc::new(AsyncSharedQueue::new()),
            device_pop_events : Arc::new(AsyncSharedQueue::new()),
            rt_join_handle : Arc::new(tokio::spawn(connection_rt)),
            rt_handle : Arc::new(RwLock::new(handle_rt)),
            rt_inner : Arc::new(rt_inner),
         }
    
    }

    pub async fn run(&mut self) -> Result<(), TrafficStarError>{
        let mut self_clone = self.clone();
        
        let creation_devices = tokio::task::spawn( async move {
            self_clone.run_create_devices().await;
        });
        self_clone = self.clone();
        let deletion_devices = tokio::spawn( async move {
            self_clone.run_delete_devices().await;
        });
        self_clone = self.clone();
        let pop_devices = tokio::spawn( async move {
            self_clone.run_pop_device().await;
        });
        let command_running = self.run_commands();
        tokio::select! {
                _v = command_running => {
                    
                },
                /*_e = creation_devices => {
                    
                }
                _e = deletion_devices => {
                    
                }*/
            }
        self.device_events.push(DeviceEvents::Stop);
        self.device_delete_events.push(DeviceDeletionEvents::Stop);
        let _ = creation_devices.await;
        let _ = deletion_devices.await;
        let _ = pop_devices.await;

        self.rt_join_handle.abort();
        Ok(())
    }

    async fn run_commands(&mut self){
        loop{
            match self.command_channel.read::<Command>().await {
                Ok(command) => {
                    if command == Command::Stop{
                        break;
                    }
                    let mut clone = self.clone();
                    tokio::task::spawn(tokio::spawn(async move {
                        let command_name = command.to_string();
                        if let Err(err) = clone.handle_command(command).await{
                            serror!("Failure handling command {}, err : {}",command_name,err);
                        }
                    }));
                },
                Err(err) => {
                    if let Some(io_error) = err.get_ioerror() && io_error.kind() == std::io::ErrorKind::UnexpectedEof{
                        sdebug!("EOF, stopping!");
                        break;
                    }
                serror!("Failed reading command, received error : {}",err)},
            };
        }
        let _ = self.command_channel.send(Command::Stopped).await;
        sdebug!("Received stop!");
    }

    async fn run_create_devices(&mut self){
        loop{
            let event = self.device_events.pop().await.unwrap();
            if event == DeviceEvents::Stop{
                sdebug!("Stopping device creation!");
                break;
            }
            if let Err(err) = self.create_devices().await{
                serror!("Received error on creating devices : {}",err);
                let mut accounts = self.accounts.write().await;
                for account in &mut *accounts{
                    if let Err(err) =  Self::delete_unknown_devices(account).await{
                        serror!("Received error on deleting unknown devices : {}",err);
                    }
                    self.device_events.push(DeviceEvents::ClearedUnknownDevices);
                }
            }
        }
    }

    async fn run_pop_device(&mut self){
        loop{
            let event = self.device_pop_events.pop().await.unwrap();
            match event{
                DevicePopperEvents::PopDevice => {
                    match self.pop_device().await{
                        Ok(_) => {
                            sdebug!("Popped a device!");
                        },
                        Err(err) => {
                            serror!("Failure popping device, error : {}. Trying again",err);
                            self.device_pop_events.push(event);
                        },
                    };
                }
                DevicePopperEvents::Stop => {
                    sdebug!("Stopping pop devices!");
                    break;
                },
            }
        }
    }

    async fn run_delete_devices(&mut self){
        let mut to_handle = 0_usize;
        loop{
            let event = self.device_delete_events.pop().await.unwrap();
            match event{
                DeviceDeletionEvents::RequestedDeleting { device, interface_name } => {
                    match self.delete_device(device, interface_name).await{
                        Ok(_v) => {to_handle += 1;},
                        Err(err) => {
                            serror!("Failed to start task to delete device, reason : {}",err);
                        },
                    };
                },
                DeviceDeletionEvents::FinishedDeleting => {
                    to_handle -= 1;
                    
                },
                DeviceDeletionEvents::Stop => break,
            };
        }
        while to_handle > 0{
            let event = self.device_delete_events.pop().await.unwrap();
            match event{
                DeviceDeletionEvents::FinishedDeleting => to_handle -= 1,
                _v => {},
            };
        }
    }

    async fn handle_command(&mut self, command : Command) -> Result<(), TrafficStarError>{
        sdebug!("Handling command : {}", command);
        match command{
            Command::Stop => panicerror!("Should never occured!"),
            Command::PopDevice => {
                self.device_pop_events.push(DevicePopperEvents::PopDevice);
            }
            Command::PoppedDevice{ device: _, interface_name: _ , keys : _} => panicerror!("Should never occured!"),
            Command::AddAccount{account} => {self.add_account(account).await?;}
            Command::DeleteDevice { device, interface_name } => self.device_delete_events.push(DeviceDeletionEvents::RequestedDeleting { device, interface_name }),
            Command::Stopped => panicerror!("Received unexpected command!"),
        };
        Ok(())
    }

    async fn delete_device(&mut self, device : MullvadDevice, interface_name : Option<String>) -> Result<tokio::task::JoinHandle<()>,TrafficStarError>{
        let accounts = self.accounts.read().await;
        for account in &*accounts{
            if account.knows_device(&device).await{

                let mut account = account.clone();
                let events_device = self.device_events.clone();
                let events_delete = self.device_delete_events.clone();
                if let Some(interface_name) = interface_name 
                    && let Err(err) = self.delete_interface(interface_name.clone(), device.get_ipv4().unwrap()).await{
                    serror!("Failure to delete interface {}, Error : {}",interface_name,TrafficStarError::from(err));
                }
               
                let handle = tokio::task::spawn(async move {
                    if let Err(err) = account.inner.push_delete_device(&device).await{
                        serror!("Failed to delete device {{{}}}, Error : {}!",device.name,err);
                        account.forget_device(&device).await;
                        while let Err(err) = Self::delete_unknown_devices(&mut account).await{
                            serror!("Failed to delete unknown devices, error : {}",err);
                        }

                    }else{
                        sdebug!("Deleted device {{{}}}",device.name);
                        account.forget_device(&device).await;

                    }

                    events_device.push(DeviceEvents::DeleteDevice);
                    events_delete.push(DeviceDeletionEvents::FinishedDeleting);

                });
                return Ok(handle);
            }
        }
        
        Err(Errors::UnknownOwnerOfDevice { device }.into())
    }


    async fn create_device(account : AsyncMullvadSlaveAccountEntry,
        device_events : Arc<AsyncSharedQueue<DeviceEvents>>,
        device_queue : Arc<AsyncSharedQueue<(MullvadDevice,WireguardKeys)>>
        ) -> Result<(), TrafficStarError>{
        loop{
            let keys = WireguardKeys::default();
            let device = account.inner.push_create_device_request(&keys).await?;
            sdebug!("Created device {{{}}}!",device.name);
            let ipaddress = device.get_ipv4().unwrap();
            let could_reserv = tokio::task::spawn_blocking(move || {
                ReservationController::instance().reserv_ipaddress(&ipaddress)
            }).await.unwrap();
            if !could_reserv{
                serror!("Mullvad device ip address {} given for {} is already reserved!", &device.get_ipv4().unwrap(), &device.name);
                loop{
                    if let Err(err) = account.inner.push_delete_device(&device).await{
                        serror!("Failed deleting device {}, error : {}",device.name, err);
                    }else{
                        sinfo!("Deleted device {} since it's address is already reserved!",&device.name);
                        break;
                    }
                }
                continue;
            }
            account.add_known_device(device.clone(), WireguardKeys { pubkey: keys.pubkey, privkey: keys.privkey }).await;
            device_events.push(DeviceEvents::CreatedDevice);
            device_queue.push((device,keys));
            break;
        }
        Ok(())
    }
    
    
    
    async fn create_devices(&mut self) -> Result<(), TrafficStarError>{
        let accounts = self.accounts.read().await;
        let mut futures = Vec::new();
        for account in &*accounts{
            for _i in account.count_known_devices().await..Self::MAX_DEVICES_ACCOUNT{
               futures.push(tokio::task::spawn(Self::create_device(account.clone(), self.device_events.clone(), self.device_queue.clone())));
            }
        }
        let mut stored_error : Option<TrafficStarError> = None;
        for future in futures{
            match future.await{
                Ok(v) => {
                    if let Err(err) = v{
                        serror!("Create device error : {}",err);
                        stored_error = Some(err);
                    }
                },
                Err(err) => {
                    serror!("Join error : {}",err);
                    return Err(TrafficStarError::msg("JoinError".into()))
                },
            }
        }
        if let Some(err) = stored_error{
            Err(err)
        }else{
            Ok(())
        }
    }
    
    
    async fn create_and_reserv_random_interface_name(&self) -> String{
        tokio::task::spawn_blocking(move || {
            ReservationController::instance().create_and_reserv_random_interface_name(Some("mv-".to_string())).to_string()
        }).await.unwrap()
    }
    
    
   async fn setup_interface(&self, interface_name: String, device: &MullvadDevice) -> Result<(), TrafficStarError> {
        let handle = self.rt_handle.write().await;
        
      

        if let Err(err) = handle.link().add(LinkWireguard::new(&interface_name).build()).execute().await {
            return Err(format!("Failed to create interface {}: {}", interface_name, err).into())
        }
           
           
        let mut links = handle
        .link()
        .get()
        .match_name(interface_name.clone())
        .execute();
        let res = links.try_next().await;
        if let Err(err) = res{
            return Err(format!("Failed to create interface {}: {}", interface_name, err).into())
        }
        let res = res.unwrap();
        if res.is_none(){
            return Err(format!("Failed to create interface {}: interface dont exist!", interface_name).into()) 
        }
        let res = res.unwrap();
        if let Err(err) = handle.address().add(res.header.index, std::net::IpAddr::V4(device.get_ipv4()?),32).execute().await {
            return Err(format!("Failed to add ip address for interface {}: {}", interface_name, err).into())
        }
        if let Err(err) = handle.address().add(res.header.index, std::net::IpAddr::V6(device.get_ipv6()?),128).execute().await {
            return Err(format!("Failed to add ip address for interface {}: {}", interface_name, err).into())
        }
        if let Err(err) = handle
        .link()
        .set(LinkUnspec::new_with_index(res.header.index).up().build())
        .execute()
        .await{
            return Err(format!("Failed to set interface up {}: {}", interface_name, err).into())
        }


        Ok(())
    }
    
    async fn create_interface(&self, device : &MullvadDevice) -> Result<String,TrafficStarError>{
        let name = self.create_and_reserv_random_interface_name().await;
        
        self.setup_interface(name.clone(), device).await?;
        Ok(name)
    }

    async fn delete_interface(&self, interface_name : String, ipv4_addr : Ipv4Addr) -> Result<(),Errors>{
        let handle = self.rt_handle.write().await;
        let mut links = handle.link().get().match_name(interface_name.clone()).execute();
        if let Ok(res) = links.try_next().await && let Some(link) = res{
            if let Err(_err) = handle.link().del(link.header.index).execute().await{
                Err(Errors::FailureDeletingInterface)
            }else{
                
                    tokio::task::spawn_blocking(move || {
                        ReservationController::instance().free_ipaddress(&ipv4_addr);
                        ReservationController::instance().free_interface(TrafficStarInterfaceName::from_str(&interface_name).unwrap());
                    }).await.unwrap();
                Ok(())
            }
        }else{
            Err(Errors::CouldntFindInterface)
        }
    }

    async fn pop_device(&mut self) -> Result<(), TrafficStarError>{
        let (device, keys) = self.device_queue.pop().await?;
        for account in &*self.accounts.read().await{
            if account.knows_device(&device).await{
                let interface_name = self.create_interface(&device).await?;

                sdebug!("Created interface with interface name {}!",interface_name);
                let _ = self.command_channel.send(Command::PoppedDevice { device, interface_name , keys}).await;
                return Ok(())
            }
        }
        
        Err(Errors::CreatedDeviceUntraced.into())
    }
    
    async fn add_account(&mut self, account : String) -> Result<(),TrafficStarError>{
        let mut lock = self.accounts.write().await;
        for i in &*lock{
            if i.inner.account_number() == account{
                serror!("Tried adding an account which is already added, skipping!");
                return Ok(())
            }
        }
        let inner = AsyncMullvadAccount::new(account).await;
        let mut entry = AsyncMullvadSlaveAccountEntry { inner : Arc::new(inner), devices : 
        Arc::new(RwLock::new(
            AsyncMullvadSlaveAccountEntryHolder { known_devices: vec![] }
        ))
        };
        Self::delete_unknown_devices(&mut entry).await?;
        lock.push(entry);
        self.device_events.push(DeviceEvents::AddedAccount);
        Ok(())
    }

    
    
    async fn delete_unknown_devices(entry : &mut AsyncMullvadSlaveAccountEntry) -> Result<(),TrafficStarError>{
        for p in entry.inner.fetch_devices().await?{
            if !entry.knows_device(&p).await{
                entry.inner.push_delete_device(&p).await?;
                sdebug!("Deleted unknown device {}!",p.name);
            }
        }
        Ok(())
    }
}
