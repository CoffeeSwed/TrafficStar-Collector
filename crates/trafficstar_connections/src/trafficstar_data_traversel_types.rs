use std::{fmt::Error, io::ErrorKind};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(strum_macros::Display, Clone, Copy, EnumIter, PartialEq, Debug, Serialize, Deserialize)]
pub enum ConnectionType {
    Internal,
    Ethernet,
    Wifi,
    FiveG,
    Starlink,
    Tunnel,
    DoubleTunnel,
    TOR,
    LocalTrafficStarProxy
}

impl TryFrom<String> for ConnectionType {
    type Error = ErrorKind;

    fn try_from(value: String) -> Result<Self, ErrorKind> {
        for p in ConnectionType::iter() {
            if p.to_string() == value {
                return Ok(p);
            }
        }

        Err(ErrorKind::InvalidData)
    }
}

impl TryFrom<ConnectionType> for Vec<u8> {
    type Error = Error;
    fn try_from(value: ConnectionType) -> Result<Self, Self::Error> {
        Ok((value as u32).to_le_bytes().to_vec())
    }
}

impl TryFrom<Vec<u8>> for ConnectionType {
    type Error = Error;
    fn try_from(v: Vec<u8>) -> Result<ConnectionType, Error> {
        for e in ConnectionType::iter() {
            let t: Vec<u8> = Vec::try_from(e)?;
            let mut ok = v.len() == t.len();
            if ok {
                for i in 0..v.len() {
                    if t[i] != v[i] {
                        ok = false;
                        break;
                    }
                }
            }

            if ok {
                return Ok(e);
            }
        }
        Err(Error)
    }
}
