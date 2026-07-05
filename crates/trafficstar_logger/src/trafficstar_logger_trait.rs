
pub trait TrafficStarStructName{
    fn struct_name() -> &'static str;
}

#[macro_export]
macro_rules! sinfo {
    ($($arg:tt)+) => ({
        let msg = format!($($arg)+);
        ::log::info!(target: <Self as trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName>::struct_name(), "{}",msg);
    });
}

#[macro_export]
macro_rules! sdebug {
    ($($arg:tt)+) => ({
        let msg = format!($($arg)+);
        ::log::debug!(target: <Self as trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName>::struct_name(), "{}",msg);
    });
}

#[macro_export]
macro_rules! serror {
    ($($arg:tt)+) => ({
        let msg = format!($($arg)+);
        ::log::error!(target: <Self as trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName>::struct_name(), "{}",msg);
    });
}

#[macro_export]
macro_rules! swarn {
    ($($arg:tt)+) => ({
        let msg = format!($($arg)+);
        ::log::warn!(target: <Self as trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName>::struct_name(), "{}",msg);
    });
}
