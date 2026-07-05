use std::sync::Arc;

use trafficstar_interface::reservation::{InterfaceReservation, Ipv4Reservation, MarkReservation};
use trafficstar_utilities::trafficstar_networking::interface::TrafficStarInterfaceName;

pub mod tor_process;
//pub mod tor_device;

#[allow(dead_code)]
#[derive(Clone)]
pub struct TorInterfaceConfig{
    pub interface_name : Arc<InterfaceReservation>,
    pub fwmark_resv : Arc<MarkReservation>,
    pub ipv4_addr : Arc<Ipv4Reservation>,
    pub out_interface : Arc<TrafficStarInterfaceName>

}

impl TorInterfaceConfig{
    pub async fn new(out_interface : TrafficStarInterfaceName) -> Self {
        let res = tokio::task::spawn_blocking(move || {            
            (
            InterfaceReservation::new(Some("tor-".into())),
            MarkReservation::new(),
            Ipv4Reservation::new()
            )
        }).await.unwrap();

        Self{
            interface_name : res.0,
            fwmark_resv : res.1,
            ipv4_addr : res.2,
            out_interface : Arc::new(out_interface)
        }
    }
}