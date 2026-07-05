
#[derive(strum_macros::EnumMessage)]
pub enum TrafficStarErrorTypes{
    #[strum(message = "Unsupported Operating System!")]
    BadOS{},
    #[strum(message = "JoinHandle returned an error on join!")]
    JoinHandleBad{error : String},
    #[strum(message = "Missed a required variable!")]
    MissingRequiredVariable{variable : String},
    #[strum(message = "A parser error occured when parsing an address!")]
    AddrParseError{inner : std::net::AddrParseError},
}