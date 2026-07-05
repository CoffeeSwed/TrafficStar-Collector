#![feature(thread_spawn_hook)]

use std::sync::Once;

use crate::trafficstar_logger::TrafficStarLogger;

pub mod trafficstar_logger;
pub mod trafficstar_logger_trait;
pub mod trafficstar_time_string_creator;
mod trafficstar_logger_record;
pub fn setup_and_use(){
    static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let res = log::set_logger(TrafficStarLogger::get_singleton());
            if let Ok(_res) = res{
                log::set_max_level(log::LevelFilter::Debug);

            }
            TrafficStarLogger::set_nick_thread(Some(trafficstar_logger::TrafficStarLoggerNick { nicks: vec![] }));
            TrafficStarLogger::set_threadhook_nick(Some(trafficstar_logger::TrafficStarLoggerNick { nicks: vec![] }));
        });
}



#[macro_export]
macro_rules! panicerror {
    ($($arg:tt)+) => ({
        let msg = format!($($arg)+);
        ::log::error!("{}", msg);
        std::process::exit(0x0100);
    });
}

