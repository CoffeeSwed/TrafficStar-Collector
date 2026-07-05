use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use ipnet::IpNet;
use once_cell::sync::OnceCell;
use trafficstar_connections::trafficstar_wireguard::{WireguardKeys, WireguardPeer};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{serror, swarn, trafficstar_logger::TrafficStarLogger};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{get_multi_runtime, get_singleton_multi, trafficstar_pipes::TrafficStarPipePairAsync, trafficstar_stored_join::StoredJoin};
use wgctrl::types::PeerConfig;


use crate::{trafficstar_async_mullvad_handler_structs::Command, trafficstar_async_mullvad_slave::AsyncMullvadSlave, trafficstar_mullvad_device::MullvadDevice, trafficstar_mullvad_relays::MullvadRelaysResponse};
#[derive(StructLoggerName)]
struct AsyncMullvadHandlerInner{
    command_channel : TrafficStarPipePairAsync,
    handle_slave : Arc<StoredJoin<()>>,
    relays : Arc<MullvadRelaysResponse>
}
static INSTANCE: OnceCell<Result<Arc<AsyncMullvadHandlerInner>,TrafficStarError>> = OnceCell::new();

impl AsyncMullvadHandlerInner{

    async fn new() -> Result<Arc<Self>,TrafficStarError>{
        
        TrafficStarLogger::mute("netlink_packet_route::link::buffer_tool".to_string(), log::Level::Warn);
        let response =
            reqwest::get(crate::trafficstar_mullvad_requests::MULLVAD_API_RELAYS).await;
        if let Err(err) = response{
            return Err(format!("{}",err).into()); 
        }
        
        let relays = match response.unwrap().json::<MullvadRelaysResponse>().await {
            Err(err) => {
                return Err(format!("{}",err).into());
            }
            Ok(arr) => {
                Arc::new(arr)
            }
        };
       
        


        let (command_pair_one, command_pair_two) = TrafficStarPipePairAsync::new_pairs().await?;
        let res = Arc::new(Self{
            command_channel : command_pair_one,
            relays,
            handle_slave : Arc::new(StoredJoin::new(std::thread::spawn(move || {
                let rt = get_multi_runtime().unwrap();
                let future = 
                rt.spawn(async {
                    let mut slave = AsyncMullvadSlave::new(command_pair_two).await;
                    if let Err(err) = slave.run().await{
                        serror!("HttpProxyError : {}",err)
                    }
                });
                rt.block_on(future).unwrap()
            })))
        });
        Ok(res)
    }


    pub async fn singleton() -> Result<Arc<Self>,TrafficStarError>{
        let res: Result<Arc<AsyncMullvadHandlerInner>, TrafficStarError> = tokio::task::spawn_blocking(|| {
            let instance: Result<Arc<AsyncMullvadHandlerInner>, TrafficStarError> = INSTANCE.get_or_init(|| {
                let part: Result<Arc<AsyncMullvadHandlerInner>, TrafficStarError> = get_singleton_multi().block_on(async move {
                    Self::new().await
                });
                part
             }).clone();
             instance
        }).await.unwrap();

        res
    }

    

    pub async fn add_account(node : Arc<Self>, account : String) -> Result<(),TrafficStarError>{
        let channel = node.command_channel.clone();
        channel.send(Command::AddAccount{account}).await?;
        Ok(())
    }

    

    pub async fn delete_device(node : Arc<Self>, device : MullvadDevice, interface_name : Option<String>) -> Result<(),TrafficStarError>{
        let channel = node.command_channel.clone();
        let _ = channel.send(Command::DeleteDevice { device, interface_name }).await;
        
        Ok(())
    }
    

    pub async fn kill(node : Arc<Self>) -> Result<(), TrafficStarError>{
        let _ = node.command_channel.clone().send(Command::Stop).await;
        node.handle_slave.join()
    }


    pub async fn get_device(node : Arc<Self>) -> Result<MullvadDeviceHolder, TrafficStarError>{
        let channel = node.command_channel.clone();
        channel.send(Command::PopDevice{}).await?;
        
        loop{
            let response = match channel.read::<Command>().await{
                Ok(v) => v,
                Err(err) => {
                    serror!("Failed reading response, got : {}",err);
                    continue;
                },
            };
            match response{
                Command::PoppedDevice { device, interface_name, keys} => {
                        let holder = MullvadDeviceHolder::new(device, interface_name ,keys,node.clone());
                        return Ok(holder)
                    },
                v => {
                    swarn!("Read unexpected command {}",v);
                }
            }
    }
    }

    pub fn get_relays(node : Arc<Self>) -> Arc<MullvadRelaysResponse>{
        node.relays.clone()
    }

}

#[derive(Clone)]
pub struct AsyncMullvadHandler{
    inner : Arc<AsyncMullvadHandlerInner>
}

///Just for the clone + legacy.
impl AsyncMullvadHandler{
    pub async fn singleton() -> Result<Self,TrafficStarError>{
        Ok(Self{
            inner : AsyncMullvadHandlerInner::singleton().await?
        })
    }
    
    pub async fn add_account(&self, account : String) -> Result<(),TrafficStarError>{
        AsyncMullvadHandlerInner::add_account(self.inner.clone(), account).await
    }

    pub async fn get_device(&self) -> Result<MullvadDeviceHolder, TrafficStarError>{
        AsyncMullvadHandlerInner::get_device(self.inner.clone()).await
    }

    pub async fn kill(&self) -> Result<(), TrafficStarError>{
        AsyncMullvadHandlerInner::kill(self.inner.clone()).await
    }

    pub fn get_relays(&self) -> Arc<MullvadRelaysResponse>{
        AsyncMullvadHandlerInner::get_relays(self.inner.clone())
    }

    pub async fn delete_device(&self, device : MullvadDevice, interface_name : Option<String>) -> Result<(),TrafficStarError>{
       AsyncMullvadHandlerInner::delete_device(self.inner.clone(), device, interface_name).await
    }
    

}


///PartialEQ only checks device, not the creator.
#[derive(StructLoggerName)]
pub struct MullvadDeviceHolder{
    device : MullvadDevice,
    interface_name : String,
    keys : WireguardKeys,
    creator : Arc<AsyncMullvadHandlerInner>,
    deleted : bool,
}

impl PartialEq for MullvadDeviceHolder {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device
    }
}

impl MullvadDeviceHolder{
    fn new(device : MullvadDevice,interface_name : String,keys : WireguardKeys, creator : Arc<AsyncMullvadHandlerInner>) -> Self{
        MullvadDeviceHolder { device, interface_name, creator, keys, deleted : false}
    }

    pub fn device(&self) -> &MullvadDevice{
        &self.device
    }

    pub fn interface_name(&self) -> &str{
        &self.interface_name
    }

    pub fn keys(&self) -> WireguardKeys{
        self.keys
    }

    pub async fn use_peer(&self, peer : WireguardPeer, fwmark : Option<u32>) -> Result<(), TrafficStarError>{
        let mut allowed_ips : Vec<IpNet> = Vec::new();
        for allowed_ip in peer.allowedips.split(","){
            allowed_ips.push(match IpNet::from_str(allowed_ip.trim()){
                Ok(v) => {
                    v},
                Err(err) => {
                    return Err(format!("Bad Allowed_Ips : {}, err : {}",&peer.allowedips, err).into())
                },
            });
        }

        let pubkey_peer = match wgctrl::types::Key::try_from(&peer.pubkey as &str){
            Ok(v) => v,
            Err(err) => {
                return Err(format!("Unable to get pubkey, err {}",err).into())
            },
        };
        
        let privkey = match wgctrl::types::Key::try_from(&*self.keys().privkey as &[u8]){
            Ok(v) => v,
            Err(err) => {
                return Err(format!("Unable to get privkey, err {}",err).into())
            },
        };
        
        let endpoint = match SocketAddr::from_str(&peer.endpoint){
            Ok(v) => v,
            Err(err) => {
                return Err(format!("Unable to get endpoint from string {}, err {}",&peer.endpoint,err).into())
            },
        };


        let config = wgctrl::types::Config {
            private_key: Some(privkey),
            listen_port: None,
            firewall_mark: fwmark,
            replace_peers: true,
            peers: vec![PeerConfig{ 
                public_key: pubkey_peer, 
                remove: false, 
                update_only: false, 
                preshared_key: None, 
                endpoint: Some(endpoint), 
                persistent_keepalive_interval: Some(Duration::from_secs(1)), 
                replace_allowed_ips: true, 
                allowed_ips }],
        };


        match wgctrl::client::Client::new(){
            Ok(mut client) => {
                if let Err(err) = client.configure_device(self.interface_name(), &config){
                        Err(format!("Error configuring device, err {}",err).into())
                }else{
                    Ok(())
                }
            },
            Err(err) => 
            Err(format!("Error getting wg-client!, err {}",err).into()),
        }
        
        
    }

    pub async fn delete(mut self) -> Result<(),TrafficStarError>{

        AsyncMullvadHandlerInner::delete_device(self.creator.clone()  , self.device.clone(), Some(self.interface_name.clone())).await?;
        self.deleted = true;
        Ok::<(),TrafficStarError>(())
        
    }
}

impl Drop for MullvadDeviceHolder{
    fn drop(&mut self) {
       if !self.deleted{
            let holder = self.creator.clone();
            let device = self.device.clone();
            let name = self.interface_name.clone();
            get_singleton_multi().spawn(async move{
                AsyncMullvadHandlerInner::delete_device(holder, device, Some(name)).await?;
                Ok::<(),TrafficStarError>(())
            });
       }
    }
}
