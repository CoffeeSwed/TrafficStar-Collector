use core::fmt;

use trafficstar_errors::{trafficstar_error_traits::TrafficStarEnumErrorTrait};

#[derive(Debug,Clone, strum_macros::EnumMessage)]
pub enum PipeErrorKind{
    FailedToRead,
    IOError
}
#[derive(Debug, Clone)]
pub struct PipeError {
    pub kind : PipeErrorKind,
    pub message : Option<String>,
}

impl From<std::io::Error> for PipeError {
    fn from(value: std::io::Error) -> Self {
        PipeError { kind: PipeErrorKind::IOError, message: Some(format!("{}",value)) }
    }
}

impl fmt::Display for PipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message.clone() {
            Some(v) => {
                match self.kind {
                    PipeErrorKind::FailedToRead => {
                        write!(f,"Failed to read, message : {}",v)
                    },
                    PipeErrorKind::IOError => {
                        write!(f,"IO Error, message : {}",v)
                    },
                }
            },
            None => {
                write!(f,"PipeError {}",self.kind.enum_variant())
            },
        }
    }
}

impl TrafficStarEnumErrorTrait for PipeError{
    fn enum_name(&self) -> &str {
        self.kind.enum_name()
    }

    fn enum_variant(&self) -> &str {
        self.kind.enum_variant()
    }

    fn enum_message(&self) -> Option<String> {
        Some(format!("{}",self))
    }
}