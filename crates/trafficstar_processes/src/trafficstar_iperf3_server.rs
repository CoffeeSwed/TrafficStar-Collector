use std::{process::Command, thread::JoinHandle};


use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::trafficstar_logger::TrafficStarLogger;

use crate::{trafficstar_iperf3_settings::Iperf3Settings, trafficstar_processes::{Process, create_process, get_line_from_process_out, kill_if_can}};
///read_timeout_ms defaults to 10 000 ms if nothing else is specified.
pub fn lookupon_iperf3_server(mut process : Process, read_timeout_ms : Option<u64>) -> Result<(),TrafficStarError>{
    let mut timeout = 10*1000;
    if let Some(read_timeout_ms) = read_timeout_ms{
        timeout = read_timeout_ms;
    }
    let handle: JoinHandle<Result<(), TrafficStarError>> = std::thread::spawn(move || {
        TrafficStarLogger::add_nick_thread("Iperf3Server".to_string());
        loop{
            match process.read_line(Some(timeout)){
                Ok(line) => {
                    if line.is_empty(){
                        return Ok(())
                    }
                },
                Err(val) => {
                    return Err(val)
                },
            }
            
        }
    });
    handle.join().unwrap()
}

#[allow(dead_code)]
pub fn start_iperf3_server(port: u16,     mut settings : Option<Iperf3Settings>) -> Result<Process, TrafficStarError> {
    if settings.is_none(){
        settings = Some(Iperf3Settings::default())
    }
    let mut command: Command = Command::new("stdbuf");
    command.arg("-oL");
    command.arg("iperf3");
    command.arg("-s");
    command.arg("--one-off");
    command.arg("-p");
    command.arg(port.to_string());

    if let Some(settings) = settings{
        command.arg("-i");
        command.arg(settings.report_interval.to_string());
        
        command.arg("--idle-timeout");
        command.arg(settings.idle_timeout_s.to_string());

        command.arg("--rcv-timeout");
        command.arg(settings.recv_timeout_ms.to_string());
    }

    let mut process = create_process(&mut command, "Iperf3_Server".to_string())?;
    let mut first_line = get_line_from_process_out(&mut process);
    if first_line.is_ok() && first_line.unwrap().contains("-----") {
        first_line = get_line_from_process_out(&mut process);
        if first_line.unwrap().contains("Server listening on") {
            return Ok(process);
        }
    }
    kill_if_can(&mut process);
    Err("Broken pipe!".into())
}
