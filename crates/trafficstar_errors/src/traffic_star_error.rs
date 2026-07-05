use std::{fmt::Debug, sync::Arc};

pub use crate::trafficstar_error_traits::TrafficStarEnumError;
use crate::{trafficstar_error_traits::TrafficStarEnumErrorTrait, trafficstar_error_types::TrafficStarErrorTypes};

#[derive(Clone, PartialEq, strum_macros::Display, Debug, Copy)]
pub enum TrafficStarErrorKind{
    NixErrno,
    IOError,
    JustMessage,
    IdentifactionMessage,
    TrafficStarEnumError,
    Empty
}


#[derive(Clone)]
pub struct TrafficStarError{
    kind : TrafficStarErrorKind,
    ioerror : Option<Arc<std::io::Error>>,
    nixerror : Option<nix::errno::Errno>,
    just_message : Option<String>,

    ///Error ID, Message
    identification_message : Option<(String,String)>,
    trafficstar_enum_error : Option<Arc<TrafficStarEnumError>>
}


impl TrafficStarError{
    
    
    pub fn io(error : Arc<std::io::Error>) -> Self{
        Self::default().set_kind(TrafficStarErrorKind::IOError).set_io(Some(error))
    }

    pub fn nix(error : nix::errno::Errno) -> Self{
        Self::default().set_kind(TrafficStarErrorKind::NixErrno).set_nix(Some(error))
    }

    pub fn msg(error : String) -> Self{
        Self::default().set_kind(TrafficStarErrorKind::JustMessage).set_msg(Some(error))
    }

    pub fn id_msg(identification : String, message : String) -> Self{
        Self::default().set_kind(TrafficStarErrorKind::IdentifactionMessage).set_id_msg(Some((identification,message)))
    }

    pub fn enums(error : Arc<TrafficStarEnumError>) -> Self{
        Self::default().set_kind(TrafficStarErrorKind::TrafficStarEnumError).set_enum(Some(error))
    }

    pub fn set_kind(&self, value: TrafficStarErrorKind) -> Self {
        let mut error = self.clone();
        error.kind = value;
        error
    }

    pub fn set_io(&self, value: Option<Arc<std::io::Error>>) -> Self {
        let mut error = self.clone();
        error.ioerror = value;
        error
    }

    pub fn set_nix(&self, value: Option<nix::errno::Errno>) -> Self {
        let mut error = self.clone();
        error.nixerror = value;
        error
    }

    pub fn set_msg(&self, value: Option<String>) -> Self {
        let mut error = self.clone();
        error.just_message = value;
        error
    }

    pub fn set_id_msg(&self, value: Option<(String,String)>) -> Self {
        let mut error = self.clone();
        error.identification_message = value;
        error
    } 
    
    pub fn set_enum(&self, value: Option<Arc<TrafficStarEnumError>>) -> Self {
        let mut error = self.clone();
        error.trafficstar_enum_error = value;
        error
    }
    
    pub fn kind(&self) -> TrafficStarErrorKind {
        self.kind
    }
    
    pub fn get_ioerror(&self) -> Option<Arc<std::io::Error>> {
        self.ioerror.clone()
    }
    
    pub fn get_nixerror(&self) -> Option<nix::errno::Errno> {
        self.nixerror
    }
    
    pub fn get_msg(&self) -> Option<&String> {
        self.just_message.as_ref()
    }
    
    pub fn get_id_msg(&self) -> Option<&(String, String)> {
        self.identification_message.as_ref()
    }
    
    pub fn get_enum(&self) -> Option<&Arc<dyn TrafficStarEnumErrorTrait + Send + Sync + 'static>> {
        self.trafficstar_enum_error.as_ref()
    }
    
    
}

impl Default for TrafficStarError{
    fn default() -> Self {
        Self { kind: TrafficStarErrorKind::Empty, ioerror: None, nixerror: None, just_message: None, identification_message: None, trafficstar_enum_error: None }
    }
}

impl Debug for TrafficStarError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrafficStarError").field("kind", &self.kind).field("ioerror", &self.ioerror).field("nixerror", &self.nixerror).field("just_message", &self.just_message).field("enum_error_string", &self.identification_message).field("trafficstarerror", match self.trafficstar_enum_error.is_some(){
            true => &"Some",
            false => &"None",
        }).finish()
    }
}


impl std::fmt::Display for TrafficStarError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("TrafficStarError{{Kind = {}", self.kind);
        if let Some(ioerror) = self.ioerror.clone(){
            result = format!("{}, IOError = {{{}}}",result,ioerror);
        }
        
        if let Some(nixerror) = self.nixerror{
            result = format!("{}, NIXError = {{{}}}",result,nixerror);
        }
        if let Some(just_message) = self.just_message.clone(){
            result = format!("{}, Message = {{{}}}",result,just_message);
        }
        if let Some(identification_message) = self.identification_message.clone(){
            result = format!("{}, ErrorIdentifcation = {{ID = {}, Message = {{{}}}}}",result,identification_message.0, identification_message.1);
        }
        if let Some(enum_error) = self.trafficstar_enum_error.clone(){
            result = format!("{}, EnumError = {{{}}}",result,enum_error as Arc<dyn TrafficStarEnumErrorTrait>);
        }
        write!(f,"{}}}",result)
    }
}

impl From<std::io::Error> for TrafficStarError {
    fn from(value: std::io::Error) -> Self {
        TrafficStarError { kind: TrafficStarErrorKind::IOError, ioerror: Some(Arc::new(value)), nixerror: None, just_message: None, identification_message: None, trafficstar_enum_error : None}
    }
}


impl From<nix::errno::Errno> for TrafficStarError {
    fn from(value: nix::errno::Errno) -> Self {
        TrafficStarError { kind: TrafficStarErrorKind::IOError, ioerror: None, nixerror: Some(value), just_message: None, identification_message : None, trafficstar_enum_error : None}
    }
}

impl From<String> for TrafficStarError {
    fn from(value: String) -> Self {
        TrafficStarError { kind: TrafficStarErrorKind::JustMessage, ioerror: None, nixerror: None, just_message: Some(value), identification_message : None, trafficstar_enum_error : None}
    }
}

impl From<&str> for TrafficStarError {
    fn from(value: &str) -> Self {
        TrafficStarError { kind: TrafficStarErrorKind::JustMessage, ioerror: None, nixerror: None, just_message: Some(value.to_string()), identification_message : None, trafficstar_enum_error : None}
    }
}


//Creates Enum Error
impl From<(String,String)> for TrafficStarError{
    fn from(value: (String,String)) -> Self {
        TrafficStarError { kind: TrafficStarErrorKind::IdentifactionMessage, ioerror: None, nixerror: None, just_message: None, identification_message: Some(value), trafficstar_enum_error : None}
    }
}

impl From<TrafficStarError> for std::io::Error{
    fn from(value: TrafficStarError) -> Self {
        match value.kind{
            TrafficStarErrorKind::IOError => {
                let val = value.ioerror.unwrap();
                std::io::Error::new(val.kind(), format!("{}",val))
            },
            _ => {
                std::io::Error::other(format!("{}",value))
            },
            
        }
    }
}

impl From<Arc<TrafficStarEnumError>> for TrafficStarError{
    fn from(value: Arc<TrafficStarEnumError>) -> Self {
        TrafficStarError::enums(value)
    }
}

impl From<Box<dyn core::error::Error>> for TrafficStarError{
    fn from(value: Box<dyn core::error::Error>) -> Self {
        Self::msg(format!("{}",value))
    }
}

impl From<tokio::task::JoinError> for TrafficStarError{
    fn from(value: tokio::task::JoinError) -> Self {
        Self::msg(format!("Tokio Join error : {}",value))
    }
}

impl From<std::net::AddrParseError> for TrafficStarError{
    fn from(value: std::net::AddrParseError) -> Self {
        Self::enums(Arc::new(TrafficStarErrorTypes::AddrParseError { inner: value }))
    }
}