use std::sync::Arc;

use tokio::{task::JoinHandle};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_utilities::{get_multi_runtime, get_singleton_multi, trafficstar_networking::interface::TrafficStarInterfaceName};

use crate::tor_device::TorDeviceConfig;

pub struct TorTunnel{
    config : Arc<TorDeviceConfig>,
    task : Arc<JoinHandle<Result<(),TrafficStarError>>>,
    
}

impl TorTunnel{
    pub async fn new(config : TorDeviceConfig) -> Result<Self, TrafficStarError>{
        let task = get_multi_runtime()?.spawn(async move{
            
            Ok::<(),TrafficStarError>(())
        });

        Ok(Self{
            config : Arc::new(config),
            task : Arc::new(task)
        })
    }

    pub fn name(&self) -> TrafficStarInterfaceName{
        self.config.interface_name.get_name()
    }
}

impl Drop for TorTunnel{
    fn drop(&mut self) {
        get_singleton_multi().spawn()
    }
}