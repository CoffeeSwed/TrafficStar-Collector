use std::collections::HashMap;
use std::path::{PathBuf};
use std::time::Duration;
use std::{fmt::Display, sync::Arc};

use async_trait::async_trait;
use colored::Color;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::net::TcpStream;
use trafficstar_connections::trafficstar_data_route::DataRoute;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_interface::LinkType;
use trafficstar_logger::{panicerror, sdebug};
use trafficstar_logger::{serror, sinfo, trafficstar_logger::TrafficStarLogger};
use trafficstar_logger_macro::{StructLoggerName};
use trafficstar_nodes::trafficstar_node_client::ClientNode;
use trafficstar_nodes::trafficstar_node_server::{ServerNode};
use trafficstar_processes::trafficstar_mullvad_browser::{MullvadBrowserRun};
use trafficstar_utilities::sink::settings::SinkSenderSettings;
use trafficstar_utilities::{get_multi_runtime};
use trafficstar_utilities::{trafficstar_semaphore::TrafficStarSemaphore};
use uuid::Uuid;

use crate::trafficstar_test_config_file::TrafficStarTestConfigFile;
use trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName;


#[async_trait]
pub trait TrafficStarTestHandlersTraits {
    async fn run_sample(&self, sample : usize, output_directory : PathBuf, endhost : String, prefix : Option<String>) -> Result<(),TrafficStarError>;

    fn total_samples(&self) -> usize;

    fn name(&self) -> &str;

    fn config(&self) -> &TrafficStarTestConfigFile;

    fn uuid(&self) -> &Uuid;
    
}

pub type TrafficStarTestHandler = dyn TrafficStarTestHandlersTraits + Send + Sync;

#[derive(StructLoggerName)]
pub struct TrafficStarTestSession {
    executors: Vec<Arc<Box<TrafficStarTestHandler>>>,
    pub output_directory : PathBuf,
}

impl Display for TrafficStarTestSession{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"TrafficStarTestSession{{total_samples = {}}}",self.total_samples())
    }
}

impl TrafficStarTestSession {    
    pub fn new(
        executors: Vec<Arc<Box<TrafficStarTestHandler>>>,
        output_directory : PathBuf
    ) ->
    Self {
        TrafficStarLogger::set_target_color(ServerNode::struct_name(), Some(Color::Blue));
        TrafficStarLogger::set_target_color(ClientNode::struct_name(), Some(Color::Magenta));
        TrafficStarLogger::set_target_color(Self::struct_name(), Some(Color::BrightRed));
        Self { executors, output_directory }
    }

    pub fn total_samples(&self) -> usize {

        let mut result : usize = 0;
        for executor in &(*self.executors){
            result += executor.total_samples();
        }
        result
    }

    pub fn run(&mut self, end_host : String, prefix : Option<String>){
        
        sinfo!("Starting tests!");
        let mut hashmap : HashMap<Uuid,usize> = HashMap::new();

        for executor in &self.executors{
            hashmap.insert(*executor.uuid(), 0);
        }

        let semaphore = TrafficStarSemaphore::default();
        let mut bar = ProgressBar::new(self.total_samples() as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "[{prefix:.cyan/blue}][{elapsed_precise}|{pos:.green}/{len:.cyan/blue}] [{wide_bar:.green/blue}]",
            )
            .unwrap()
            .progress_chars("=>-"),
        );

        bar.set_prefix("TestSession");
        bar.set_message("Loading tests :");
        let lock = TrafficStarLogger::lock();
        let mp = match TrafficStarLogger::get_progress_bars(){
            Some(v) => v,
            None => {
                let mp = Arc::new(MultiProgress::new());
                TrafficStarLogger::set_progress_bars(mp.clone());
                mp
            },
        };
        bar = mp.add(bar);
        bar.enable_steady_tick(Duration::from_secs(1));
        bar.force_draw();
        drop(lock);
        let mut total_samples_performed = 0;
        while !hashmap.is_empty(){
            for executor in &self.executors{
                if let Some(number) = hashmap.get(executor.uuid()){
                    if *number == executor.total_samples(){
                        hashmap.remove_entry(executor.uuid());
                    }else{
                        let cur_num = hashmap.get_mut(executor.uuid()).unwrap();
                        let number = *cur_num;
                        *cur_num += 1;

                        total_samples_performed += 1;
                        let guard = semaphore.lock(executor.config().test_parameters.parallel);
                        let executor_clone = executor.clone();
                        let executor_name_clone = executor.name().to_string();
                        let test_name_clone = executor.config().test_parameters.name.clone().unwrap_or("None".to_string());
                        let bar_clone = bar.clone();
                        let output_directory_clone = self.output_directory.clone();
                        let end_host = end_host.clone();
                        let prefix = prefix.clone();
                        let total_samples_performed = total_samples_performed;
                        std::thread::spawn(move || {
                           TrafficStarLogger::set_nick_thread(
                                Some( vec![executor_name_clone,test_name_clone,total_samples_performed.to_string()].into()));
                            TrafficStarLogger::set_threadhook_nick(
                                TrafficStarLogger::get_nick_thread()
                            );
                            
                            let runtime = match get_multi_runtime(){
                                Ok(v) => v,
                                Err(err) => {
                                    panicerror!("Couldn't create runtime, error : {}",err);
                                },
                            };

                                sinfo!("Started test {}!",total_samples_performed);
                                if let Err(err) = runtime.block_on(executor_clone.run_sample(number,output_directory_clone,end_host.clone(), prefix.clone())){
                                    serror!("Failed test, error reason : {}",err);
                                }
                                drop(guard);
                                bar_clone.inc(1);

                             }
                        );
                    }
                }
            }
        }
        
        semaphore.wait_to(0);
        bar.finish();
        mp.remove(&bar);
    }


    pub async fn run_sink(sink_params : SinkSenderSettings, route : DataRoute, end_host : &str, output_directory : PathBuf, prefix : Option<String>) -> Result<(), TrafficStarError>{
        let tcp_stream = TcpStream::connect(end_host).await?;
             let mut node = ClientNode::init_client(tcp_stream, route.clone(), 
            output_directory.clone(),
            match TrafficStarLogger::get_nick_thread(){
                Some(v) => Some(v.nicks.clone()),
                None => None,
            }
                ).await?;
            
        node.run_client_sink(prefix.unwrap_or("sink".into()), &sink_params, None,true).await?;
        Ok(())
    }

    pub async fn run_browser(browser_run : &MullvadBrowserRun, 
        route : DataRoute, 
        end_host : String, 
        output_directory : PathBuf, 
        link_type : LinkType, 
        prefix : Option<String>) -> Result<(), TrafficStarError>{
        let timeout = browser_run.max_progress_time;
        sdebug!("Connecting to the server!");
        let tcp_stream = TcpStream::connect(end_host).await?;
        sdebug!("Connected to the server!");
        let mut node = ClientNode::init_client(tcp_stream, route.clone(), 
        output_directory.clone().join("mullvad"),
        match TrafficStarLogger::get_nick_thread(){
                Some(v) => Some(v.nicks.clone()),
                None => None,
            }
            ).await?;
        sdebug!("Made client!");

        sdebug!("Max time : {:?}",timeout);
        
        node.run_client_browser(prefix.unwrap_or("mullvad-browser".into()),
         browser_run, 
         None, 
         Some(link_type), 
         true).await
        }
}