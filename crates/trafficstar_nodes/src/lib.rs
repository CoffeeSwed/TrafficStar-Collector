pub mod trafficstar_node_client;
pub mod trafficstar_node_server;
pub mod trafficstar_nodes;

#[cfg(test)]
#[allow(unused)]
mod tests{
    /*    use log::{error, info};
    use trafficstar_connections::trafficstar_data_route::DataRoute;
    use trafficstar_errors::traffic_star_error::TrafficStarError;
    use trafficstar_logger::trafficstar_logger::{TrafficStarLogger, TrafficStarLoggerNick};
    use trafficstar_processes::trafficstar_iperf3_client::Iperf3ClientSettings;

    use crate::{trafficstar_node_client::{ClientNode}, trafficstar_node_server::{ServerNode}};

    #[warn(unused_unsafe)]
    pub fn setup(this_name : &str, child_name : Option<&'static str>){
            trafficstar_logger::setup_and_use();
            let our_nicks = TrafficStarLoggerNick{
                nicks : vec![this_name.to_string()]
            };
            let mut their_nicks = our_nicks.clone();
            if let Some(child_name) = child_name{
                their_nicks.nicks.push(child_name.to_string());
            }
            TrafficStarLogger::set_nick_thread(
                Some(our_nicks));
            TrafficStarLogger::set_threadhook_nick(
                Some(their_nicks));
        /*
                    TrafficStarLogger::set_nick_thread(
                Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick 
                { nicks: vec!["CLIENT".to_string()] }));
            TrafficStarLogger::set_threadhook_nick(
                Some(trafficstar_logger::trafficstar_logger::TrafficStarLoggerNick 
                { nicks: vec!["CLIENT".to_string(),"CHILD".to_string()] }));
                 */

                
    }

    fn run_nodes_client_iperf3(dest : SocketAddr) -> Result<(), TrafficStarError>{
        let ip = trafficstar_utilities::fetch_public_ip()+":"+&dest.port().to_string();
        let tcp_stream = TcpStream::connect(&ip)?;
        info!("Connected to {}",ip);
        
        let route = DataRoute::from_default_interface()?;
        info!("Created DataRoute!");
        
        let mut node = ClientNode::init_client(tcp_stream, route, 
            trafficstar_tester::replace_create_temp_dir("run_nodes_client")?,
            None
        )?;
        info!("Created client node!");
        /*run_client_iperf3(&mut node, "run_nodes_test".into(),
        Iperf3ClientSettings{
            time: None,
            bandwidth: Some("16000/2".into()),
            num: Some("8K".into()),
            i: None,
            k: None,
            udp: None,
            length: Some("1000".into()),
            window: None,
            c: None,
            allowed_droppeds : None
        }, None)?;*/
        node.run_client_iperf3( "run_nodes_test".into(),
        Iperf3ClientSettings{
            time: Some(10),
            bandwidth: None,
            num: None,
            i: None,
            k: None,
            udp: None,
            length: Some("1000".into()),
            window: None,
            c: None,
            allowed_droppeds : Some(u64::MAX)
        }, None)?;
        info!("Ran client like we should!");
        

        Ok(())
    }

    fn run_nodes_server_iperf3(listener : TcpListener) -> Result<(), TrafficStarError>{
        let route = DataRoute::from_default_interface()?;
        info!("Created DataRoute!");

        let mut node = ServerNode::init_server(listener, route, 
            trafficstar_tester::replace_create_temp_dir("run_nodes_server")?);
        info!("Created server node!");

        node.run_server(true).join();
        info!("Ran server like it should!");
        Ok(())
    }

    #[test]
    fn run_nodes_iperf3() {
        info!("Running nodes and exchanging traversel types!");
        
        let listener = TcpListener::bind("0.0.0.0:5201").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            setup("Server", Some("Fork"));
            run_nodes_server_iperf3(listener)
        });
        
        let client = std::thread::spawn(move || {
            setup("Client", Some("Fork"));
            run_nodes_client_iperf3(addr)
        });
        
        if let Err(client) = client.join().unwrap(){
            error!("Received error from client : {}",client);
        }

        if let Err(server) = server.join().unwrap(){
            error!("Received error from server : {}",server);
        }
    
    }*/
}