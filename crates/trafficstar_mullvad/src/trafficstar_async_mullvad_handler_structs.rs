use std::sync::{Arc};

use serde::{Deserialize, Serialize};
use trafficstar_connections::trafficstar_wireguard::WireguardKeys;
use trafficstar_errors::{traffic_star_error::{TrafficStarError}};

use crate::{trafficstar_mullvad_device::MullvadDevice};

#[derive(Deserialize,Serialize,strum_macros::Display, Clone, PartialEq)]
pub enum Command{
    Stop,
    Stopped,
    DeleteDevice{device : MullvadDevice, interface_name : Option<String>},
    PopDevice,
    PoppedDevice{device : MullvadDevice, interface_name : String, keys : WireguardKeys},
    AddAccount{account : String}
}

#[derive(Deserialize,Serialize,strum_macros::EnumMessage,Clone)]
pub enum Errors{
    #[strum(message = "Created device is untraced so the keys are unknown!")]
    CreatedDeviceUntraced,
    #[strum(message = "The message read was of unexpected Type")]
    UnexpectedCommunicationMessageType{expected : Option<String>, received : String},
     #[strum(message = "The communication channel  was dropped ")]
    CommunicationChannelDropped,
    #[strum(message = "Uknown owner of device!")]
    UnknownOwnerOfDevice{device : MullvadDevice},
    #[strum(message = "Failed to create interface!")]
    FailedToCreateInterface,
    #[strum(message = "Failed to find interface!")]
    CouldntFindInterface,
    #[strum(message = "Failed to delete interface!")]
    FailureDeletingInterface,
}

impl From<Errors> for TrafficStarError{
    fn from(value: Errors) -> Self {
        TrafficStarError::enums(Arc::new(value))
    }
}

#[derive(Deserialize,Serialize,strum_macros::Display, Clone,Copy, PartialEq)]
pub enum DeviceEvents{
    CreatedDevice,
    DeleteDevice,
    AddedAccount,
    ClearedUnknownDevices,
    Stop,
}



#[derive(Deserialize,Serialize,strum_macros::Display, Clone, PartialEq)]
pub enum DeviceDeletionEvents{
    RequestedDeleting{device : MullvadDevice, interface_name : Option<String>},
    FinishedDeleting,
    Stop
}


#[derive(Deserialize,Serialize,strum_macros::Display, Clone, PartialEq)]
pub enum DevicePopperEvents{
    PopDevice,
    Stop
}


