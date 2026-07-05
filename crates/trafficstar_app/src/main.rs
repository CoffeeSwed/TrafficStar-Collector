use clap::{Arg, Command};
use directories::BaseDirs;
use log::{error, info};

use trafficstar_logger::{trafficstar_logger::TrafficStarLogger};
use trafficstar_utilities::get_singleton_multi;
use chrono::Local;


use crate::{client::Client, configurations::Configuration, server::Server};
pub mod configurations;
pub mod client;
pub mod server;

fn main() {
    let set_logger_res = log::set_logger(TrafficStarLogger::get_singleton());
    if let Err(err) = set_logger_res {
        println!("Couldn't set logger, received error : {}", err);
    } else {
        log::set_max_level(log::LevelFilter::Debug);
        if let Some(base_dirs) = BaseDirs::new(){
            let log_file = base_dirs.data_local_dir();
            let log_file = log_file.join("TrafficStar");
            if !log_file.exists()
                && let Err(err) = std::fs::create_dir_all(&log_file){
                error!("Couldn't create directory to place log file in, error : {}",err);
            }else{
                let date = &Local::now().format("%Y-%m-%d-%H:%M:%S").to_string();
                let log_file = log_file.join(format!("{}.txt",date));
                if let Err(err) = TrafficStarLogger::set_output_file(Some(&log_file), Some(true)){
                    error!("Received error setting new Log File : {}",err);
                }else{
                    info!("Log file is : {:?}",log_file);
                }
            }
        }else{
            error!("Couldn't find where to place Log Files!");
        }
    }
    get_singleton_multi();

    let arguments =  Command::new("TrafficStar").version(env!("CARGO_PKG_VERSION"))
        .author("CoffeeSwed <CoffeeSwed@proton.me>")
        .about("Setup for performing traffic analysis.")
        .version("0.0.1")
    .arg(Arg::new("type").long("instance-type").short('t').help("Start as a server or client").value_names(vec!["Server Or Client"]).default_missing_value("Client"));

    let arguments = Configuration::add_arguments(arguments).get_matches();

    if let Some(instace_type) = arguments.get_one::<String>("type"){
        if instace_type.to_ascii_lowercase().starts_with("s"){
            info!("Instance type Server");
            let server = match Server::new(arguments){
                Ok(v) => v,
                Err(err) => {
                    error!("Failed generating client, error : {}!",err);
                    return;
                },
            };
            if let Err(err) = server.run(){
                error!("Server returned error : {}",err);
            }
        }else{
            info!("Instance type Client");
            let client = match Client::new(arguments){
                Ok(v) => v,
                Err(err) => {
                    error!("Failed generating client, error : {}!",err);
                    return;
                },
            };
            if let Err(err) = client.run(){
                error!("Client gave error : {}",err)
            }
        }
    }


}   


