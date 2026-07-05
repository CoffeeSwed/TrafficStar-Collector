use std::{
    collections::HashMap, io::{ErrorKind, Write}, process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio}, sync::Mutex, thread::JoinHandle, time::Duration
};


use futures::{AsyncReadExt};
use serde::{Deserialize, Serialize};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{panicerror};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_utilities::{create_multi_runtime, trafficstar_files::file_handler::FileHandler, trafficstar_pipes::{TrafficStarPipePair, TrafficStarPipePairAsync}};

#[derive(Serialize,Deserialize,PartialEq, strum_macros::Display)]
enum ProccessCommunication{
    ///Output slash result does not include the stop_at part. 
    /// If stop_at not given, it will read as much is available at next ready read. EOF is always treated as being part the stop_at
    RequestReadStdOut{timeout : u64, stop_at : Option<Vec<u8>>},

    ///Output slash result does not include the stop_at part. 
    /// If stop_at not given, it will read as much is available at next ready read. EOF is always treated as being the stop_at
    RequestReadStdErr{timeout : u64, stop_at : Option<Vec<u8>>},
    RequestGiveInput{input : Vec<u8>},
    Read{read : Vec<u8>},
    CouldNotRead{reason : String},
    CouldNotWrite{reason : String},
    Wrote,
    Stop
}

#[allow(dead_code)]
#[derive(StructLoggerName)]
pub struct Process {
    handle: Mutex<Option<Child>>,
    pub name : String,
    pub info : HashMap<String, String>,
    communicator_handle : JoinHandle<Result<(),TrafficStarError>>,
    communication : TrafficStarPipePair,
}

const DEFAULT_VEC_ALLOC : usize = 4096;

impl Process {
    #[allow(unused_variables)]
    pub fn new(
        mut handle : Child, name : String) -> Result<Process, TrafficStarError>{
        
        let stdout = handle.stdout.take();
        let stdin = handle.stdin.take();
        let stderr = handle.stderr.take();
        let (our_pair, their_pair) = TrafficStarPipePair::new_pairs()?;
        
        

        Ok(Process {
            handle: Mutex::new(Some(handle)),
            name,
            info : HashMap::new(),
            communication : our_pair,
            communicator_handle : std::thread::spawn(move || {
                let rt = create_multi_runtime()?;
                rt.block_on(async move{
                    Self::communicator_procces(stdout, stderr, stdin, TrafficStarPipePairAsync::try_from(their_pair)?).await
                })
            })
        })
    }

    ///Reads from stdout
    pub fn read_line(&mut self, timeout_ms : Option<u64>) -> Result<String, TrafficStarError>{
        self.communication.send(ProccessCommunication::RequestReadStdOut { timeout: timeout_ms.unwrap_or(u64::MAX), stop_at: Some(vec![b'\n']) })?;
        match self.communication.read::<ProccessCommunication>()?{
            ProccessCommunication::Read { read } => {
                 match String::from_utf8(read){
                    Ok(mut v) => match v.ends_with("\n"){
                        true => {
                            let _ = v.split_off(v.len()-1);
                            Ok(v)
                        },
                        false => Ok(v),
                    },
                    Err(err) =>  {
                        Err(format!("Parse error : {}",err).into())
                    },
                }

                
            },
            ProccessCommunication::CouldNotRead { reason } => Err(reason.into()),
            _v => panicerror!("bad response"),
        }
    }

    ///Reads from stderr
    pub fn read_line_err(&mut self, timeout_ms : Option<u64>) -> Result<String, TrafficStarError>{
        self.communication.send(ProccessCommunication::RequestReadStdErr { timeout: timeout_ms.unwrap_or(u64::MAX), stop_at: Some(vec![b'\n']) })?;
        match self.communication.read::<ProccessCommunication>()?{
            ProccessCommunication::Read { read } => {
                 match String::from_utf8(read){
                    Ok(v) => Ok(v),
                    Err(err) =>  {
                        Err(format!("Parse error : {}",err).into())
                    },
                }

                
            },
            ProccessCommunication::CouldNotRead { reason } => Err(reason.into()),
            _v => panicerror!("bad response"),
        }
    }

    

    pub fn write_bytes(&mut self, bytes : Vec<u8>) -> Result<(), TrafficStarError>{
        self.communication.send(ProccessCommunication::RequestGiveInput { input: bytes })?;
        match self.communication.read::<ProccessCommunication>()?{
            ProccessCommunication::CouldNotWrite { reason } => Err(reason.into()),
            ProccessCommunication::Wrote => Ok(()),
            _v => {
                panicerror!("Bad response!");
            }
        }
    }


    async fn read_part(file : &mut FileHandler, buffer : &mut Vec<u8>, stop_at : Option<Vec<u8>>) -> Result<usize, TrafficStarError>{
        let mut char = vec![0_u8];
        let mut read = 0_usize;
        if let Some(stop_at) = stop_at{
            buffer.clear();
            loop{
                match file.read(&mut char).await{
                    Ok(v) => {
                        if v == 1{
                            read += 1;
                            buffer.push(char[0]);
                        }else{
                            return Err("EOF".into());
                        }
                    },
                    Err(err) => {
                        if read == 0{
                            return Err(format!("Received error : {}",err).into())
                        }else{
                            break;
                        }
                    },
                };
                if buffer.ends_with(&stop_at){
                    break;
                }
            }
            Ok(read)
        }else{
            buffer.resize(file.available_read(), 0_u8);
            match file.read( buffer).await{
                Ok(v) => {
                    Ok(v)
                },
                Err(err) => {
                    Err(format!("Received error : {}",err).into())
                },
            }
        }

    }

    async fn communicator_procces(
        stdout : Option<ChildStdout>, 
        stderr : Option<ChildStderr>, 
        stdin : Option<ChildStdin>,
        mut communication : TrafficStarPipePairAsync) -> Result<(),TrafficStarError>{
        let mut buffer: Vec<u8> = Vec::with_capacity(DEFAULT_VEC_ALLOC);

        let stdout = match stdout{
            Some(v) => Some(FileHandler::new(v.into())?),
            None => None,
        };
        let stderr = match stderr{
            Some(v) => Some(FileHandler::new(v.into())?),
            None => None,
        };
        
        loop{
            match communication.read::<ProccessCommunication>().await?{
                ProccessCommunication::RequestReadStdOut {timeout ,stop_at } => {
                    if let Some(mut out) = stdout.clone(){
                        let read_future = match tokio::time::timeout(Duration::from_millis(timeout),Self::read_part(&mut out, &mut buffer, stop_at)).await{
                            Ok(v) => v,
                            Err(_) => {
                                communication.send(ProccessCommunication::CouldNotRead { reason: "Timeout".into()}).await?;
                                continue;
                            },
                        };

                        match read_future{
                            Ok(v) => communication.send(ProccessCommunication::Read { read: buffer[0..v].to_vec() }).await?,
                            Err(err) => {
                                communication.send(ProccessCommunication::CouldNotRead { reason: format!("{}",err) }).await?;
                            },
                        };
                    }else{
                        communication.send(ProccessCommunication::CouldNotRead { reason: "Broken pipe".into() }).await?;
                    }
                },
                
                ProccessCommunication::RequestReadStdErr {timeout,stop_at } => {
                    if let Some(mut out) = stderr.clone(){
                        let read_future = match tokio::time::timeout(Duration::from_millis(timeout),Self::read_part(&mut out, &mut buffer, stop_at)).await{
                            Ok(v) => v,
                            Err(_) => {
                                communication.send(ProccessCommunication::CouldNotRead { reason: "Timeout".into()}).await?;
                                continue;
                            },
                        };


                        match read_future{
                            Ok(v) => communication.send(ProccessCommunication::Read { read: buffer[0..v].to_vec() }).await?,
                            Err(err) => {
                                communication.send(ProccessCommunication::CouldNotRead { reason: format!("{}",err) }).await?;
                            },
                        };
                    }else{
                        communication.send(ProccessCommunication::CouldNotRead { reason: "Broken pipe".into() }).await?;
                    }
                },
                ProccessCommunication::RequestGiveInput { input } => {
                    if let Some(mut out) = stdin.as_ref(){
                        match out.write_all(&input){
                            Ok(_) =>  {
                                match out.flush(){
                                    Ok(_) => communication.send(ProccessCommunication::Wrote).await?,
                                    Err(err) => {
                                        communication.send(ProccessCommunication::CouldNotWrite { reason: format!("Flush error {}",err) }).await?;
                                    },
                                };
                                
                            },
                            Err(err) => {
                                communication.send(ProccessCommunication::CouldNotWrite { reason: format!("{}",err) }).await?;
                            },
                        };
                    }else{
                        communication.send(ProccessCommunication::CouldNotWrite { reason: "Broken pipe".into() }).await?;
                    }

                },
                _v => {
                    break;
                },
            }
        }
        Ok(())
    }
}

impl Drop for Process{
    fn drop(&mut self) {
        kill_if_can(self);
        let _ = wait_with_output(self);
        let _ = self.communication.send(ProccessCommunication::Stop);
    }
}


pub fn wait_with_output(process: &mut Process) -> Result<std::process::Output, std::io::Error> {
    if let Some(child) = process.handle.lock().unwrap().take() {
        child.wait_with_output()
    } else {
        Err(std::io::Error::other(
            "Process already waited on",
        ))
    }
}


pub fn get_line_from_process_out(handle: &mut Process) -> Result<String, TrafficStarError> {
    handle.read_line(None)
}




pub fn get_line_from_process_err(handle: &mut Process) -> Result<String, ErrorKind> {
    let line = &mut handle.read_line_err(None);
    match line{
        Ok(v) => Ok(v.to_string()),
        Err(_) => Err(ErrorKind::Other),
    }

}

pub fn create_process(command: &mut Command, name : String) -> Result<Process,TrafficStarError> {
    command.stderr(Stdio::piped());
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    let child = command.spawn()?;
    /*
    let output = child.stdout.take().unwrap();
    let bufreader_out = BufReader::new(output);
    let outerr = child.stderr.take().unwrap();
    let buferr_out = BufReader::new(outerr);
    */
    Process::new(child, name)
}

#[allow(dead_code)]
pub fn kill_if_can(process: &mut Process) {
    let mut lock = process.handle.lock().unwrap();
    if let Some(mut child) = lock.take() {
        child.kill().unwrap();
        *lock = Some(child);
    }
}


#[allow(dead_code)]
pub fn send_ctrlc(process: &mut Process) {
    let mut lock = process.handle.lock().unwrap();

    if let Some(child) = lock.take() {
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(child.id() as i32),
            nix::sys::signal::Signal::SIGINT,
        )
        .expect("terminating failed!");
        *lock = Some(child);
    }
}
