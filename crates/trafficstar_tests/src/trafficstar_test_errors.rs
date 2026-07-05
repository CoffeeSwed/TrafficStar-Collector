use std::fmt::{Display, Formatter};

pub enum TrafficStarTestErrorKind{
    VPN,
    BADCONFIG,
    IO,
    TOR,
    UNKNOWN
}
pub struct TrafficStarTestError{
    pub kind : TrafficStarTestErrorKind,
    pub reason : Option<String>,
    pub message : Option<String>,
}

impl Display for TrafficStarTestError{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message{
            write!(f, "{}", message)
        }else {
            if let Some(reason) = &self.reason {
                match self.kind {
                    TrafficStarTestErrorKind::VPN => {
                        write!(f, "TrafficStarTest VPN Error: {}", reason)
                    }
                    TrafficStarTestErrorKind::BADCONFIG => {
                        write!(f, "TrafficStarTest BADCONFIG Error: {}", reason)
                    }
                    TrafficStarTestErrorKind::IO => {
                        write!(f, "TrafficStarTest IO Error: {}", reason)
                    }
                    TrafficStarTestErrorKind::UNKNOWN => {
                        write!(f, "TrafficStarTest UNKNOWN Error: {}", reason)
                    }
                    TrafficStarTestErrorKind::TOR => write!(f, "TrafficStarTest TOR Error: {}", reason),
                }
            } else {
                match self.kind {
                    TrafficStarTestErrorKind::VPN => {
                        write!(f, "TrafficStarTest VPN Error")
                    }
                    TrafficStarTestErrorKind::BADCONFIG => {
                        write!(f, "TrafficStarTest BADCONFIG Error")
                    }
                    TrafficStarTestErrorKind::IO => {
                        write!(f, "TrafficStarTest IO Error")
                    }
                    TrafficStarTestErrorKind::UNKNOWN => {
                        write!(f, "TrafficStarTest UNKNOWN Error")
                    }
                    TrafficStarTestErrorKind::TOR => write!(f,"TrafficStarTest TOR Error"),
                }
            }
        }
    }
}

