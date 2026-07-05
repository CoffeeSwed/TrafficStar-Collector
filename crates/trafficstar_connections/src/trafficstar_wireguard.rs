
use serde::{Deserialize, Serialize};
use wireguard_keys::{Privkey, Pubkey};

#[derive(thiserror::Error, Debug, Clone)]
pub enum ErrorWireguard {
    #[error("Couldn't set fwmark!")]
    CouldntSetFWMark,
}
#[derive(Serialize,Deserialize,Clone,Copy, PartialEq)]
///Keys are base_64 encoded!
pub struct WireguardKeys{
    pub pubkey : Pubkey,
    pub privkey : Privkey
}

impl Default for WireguardKeys{
    fn default() -> Self {
        let private_key = wireguard_keys::Privkey::generate();
        let pub_key = private_key.pubkey();
        Self { pubkey: pub_key, privkey: private_key }
    }
}

#[derive(Clone)]
///Represents the [Interface] section of a wg-quick config file!
pub struct WireguardInterface{
    pub privkey : String,
    pub address : String,
    pub dns : Option<String>,

}

#[derive(Clone)]
///Represents the [Peer] section of a wg-quick config file
pub struct WireguardPeer{
    pub pubkey : String,
    pub allowedips : String,
    pub endpoint : String
}

