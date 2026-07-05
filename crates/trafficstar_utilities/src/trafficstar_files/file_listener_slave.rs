use std::{collections::HashMap, fs::File, io::Read, os::fd::{AsRawFd, RawFd}, sync::{Arc, RwLock, atomic::AtomicBool}};

use colored::Color;
use crossbeam_queue::SegQueue;
use nix::{fcntl::{self, FcntlArg, OFlag}, poll::PollTimeout, sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags}};
use trafficstar_logger::{panicerror, sdebug, serror, trafficstar_logger::TrafficStarLogger, trafficstar_logger_trait::TrafficStarStructName};
use trafficstar_logger_macro::StructLoggerName;
use uuid::Uuid;

use crate::{trafficstar_files::{epoll_events, file_listener::{FileListenerEntry, FileListenerEntryReadBuffer, FileListenerEntryWriteWrapper}, file_signals::{FileListenerCommunication, FileListenerSignalTypes}}, trafficstar_pipes::TrafficStarPipePair};

#[derive(Clone, strum_macros::Display)]
enum FileListenerSlaveState{
    Listening,
    ReadCommand,
}

#[derive(Clone)]
struct SlaveEntryInner{
    uuid : Arc<Uuid>,
    inner_read : Arc<FileListenerEntryReadBuffer>,
    inner_write : Arc<FileListenerEntryWriteWrapper>,
}

#[derive(Clone)]
struct SlaveEntry{
    file : Arc<File>,
    entries : Arc<RwLock<Vec<Arc<SlaveEntryInner>>>>,
    is_write_registered : Arc<AtomicBool>,
    is_read_registered : Arc<AtomicBool>,
}

impl SlaveEntry{
    #[allow(unused)]
    pub fn get_entry(&self, uuid : Uuid) -> Option<Arc<SlaveEntryInner>>{
        for entry in &*self.entries.write().unwrap(){
            if *entry.uuid == uuid{
                return Some(entry.clone())
            }
        }
        None
    }

    
    pub fn remove_entry(&self, uuid : Uuid) -> Option<Arc<SlaveEntryInner>>{
        let mut lock = self.entries.write().unwrap();
        lock.iter().position(|x| *x.uuid == uuid).map(|v| lock.remove(v))
        
    }
}

#[derive(StructLoggerName)]
pub struct FileListenerSlave{
    entries : HashMap<RawFd,Arc<SlaveEntry>>, 
    read_pipe_fd : RawFd,
    communication : TrafficStarPipePair,
    state : FileListenerSlaveState,
    epoll : Epoll,
    epoll_buffer : Vec<EpollEvent>,
    queue : Arc<SegQueue<FileListenerCommunication>>,
    show_logs : bool
}

impl FileListenerSlave{
    const EPOLL_BUFFER_ELEMENTS : usize = 512;
    const READ_BUFFER_SIZE : usize = 32768;

    pub fn start(communication : TrafficStarPipePair, queue : Arc<SegQueue<FileListenerCommunication>>){
        
        std::thread::spawn(move || {
            TrafficStarLogger::set_nick_thread(None);
            let epoll_fd = Epoll::new(EpollCreateFlags::empty()).unwrap();
            if epoll_fd.add(communication.input.file_id(),  EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR,communication.input.file_id().as_raw_fd() as u64)).is_err(){
                panicerror!("Cant create epoll event listener!");
            }
            let epoll_buffer : Vec<EpollEvent> = vec![EpollEvent::empty(); Self::EPOLL_BUFFER_ELEMENTS];
            
            let mut res = FileListenerSlave{
                read_pipe_fd : communication.input.file_id().as_raw_fd(),
                entries : HashMap::new(),
                communication,
                state : FileListenerSlaveState::Listening,
                epoll : epoll_fd,
                epoll_buffer,
                queue,
                show_logs : false,
            };

            if !res.show_logs{
                TrafficStarLogger::mute(FileListenerSlave::struct_name().to_string(), log::Level::Debug);
                TrafficStarLogger::mute(FileListenerSlave::struct_name().to_string(), log::Level::Warn);
            }else{
                TrafficStarLogger::set_target_color(Self::struct_name(), Some(Color::AnsiColor(18)));
                sdebug!("File Listener Slave Started!");

            }
            
            res.run();
            
        });
    }

    #[allow(clippy::map_clone)]
    fn get_registered(&self, file : &Arc<File>) -> Option<Arc<SlaveEntry>>{
        self.entries.get(&file.as_raw_fd()).map(std::clone::Clone::clone)
    }

    fn epoll_file(&self, entry : Arc<SlaveEntry>, get_write_events : bool, get_read_events : bool){
        let is_write_regristered = entry.is_write_registered.load(std::sync::atomic::Ordering::Acquire);
        let is_read_registered = entry.is_read_registered.load(std::sync::atomic::Ordering::Acquire);
        if (get_write_events != is_write_regristered) || (get_read_events != is_read_registered){
            let mut events = EpollEvent::new(match get_write_events {
                true => {
                    match get_read_events{
                        true => 
                    //Write : true, read : true
                    EpollFlags::EPOLLIN
                    | EpollFlags::EPOLLERR 
                    | EpollFlags::EPOLLOUT
                    | EpollFlags::EPOLLHUP
                    | EpollFlags::EPOLLET,
                        false => 
                    //Write : true, read : false
                     EpollFlags::EPOLLOUT
                    | EpollFlags::EPOLLERR 
                    | EpollFlags::EPOLLHUP 
                    | EpollFlags::EPOLLET,
                    }
                },
                false => {
                    match get_read_events{
                        true => 
                    //Write : false, read : true
                    EpollFlags::EPOLLIN 
                    | EpollFlags::EPOLLERR 
                    | EpollFlags::EPOLLHUP
                    | EpollFlags::EPOLLET,
                    //Write : false, read : false
                        false => EpollFlags::EPOLLET
                                | EpollFlags::EPOLLERR 
,
                    }
                },
            }, entry.clone().file.as_raw_fd() as u64);
            
            if !(is_read_registered || is_write_regristered) && (get_write_events || get_read_events){
                if get_write_events || get_read_events{
                    //sdebug!("Added epoll for {}!",entry.file.as_raw_fd());
                    self.epoll.add(entry.file.clone(), events).unwrap();
                }
            }else{
                if get_read_events || get_write_events{
                    self.epoll.modify(entry.file.clone(), &mut events).unwrap();
                }else{
                    //sdebug!("Epoll delete {}!",entry.file.as_raw_fd());
                    self.epoll.delete(entry.file.clone()).unwrap();
                }
            }
        }
        
        entry.is_write_registered.store(get_write_events, std::sync::atomic::Ordering::Release);
        entry.is_read_registered.store(get_read_events, std::sync::atomic::Ordering::Release);
        
        
    }
    
    fn create_push_register_entry(&mut self, file : Arc<File>){
        let v = Arc::new(SlaveEntry{
                    file : file.clone(),
                    entries : Arc::new(RwLock::new(Vec::new())),
                    is_write_registered : Arc::new(AtomicBool::new(false)),
                    is_read_registered : Arc::new(AtomicBool::new(false)),
                });
        self.epoll_file(v.clone(), true, true);

        self.entries.insert(file.as_raw_fd(),v);
        

        
        ////sdebug!("Registered file : {}",file.as_raw_fd());
        let flags = OFlag::from_bits_truncate(fcntl::fcntl(file.clone(), FcntlArg::F_GETFL).unwrap());
        let new_flags = flags | OFlag::O_NONBLOCK;
        fcntl::fcntl(file.clone(), FcntlArg::F_SETFL(new_flags)).unwrap();
        ////sdebug!("Set file {} as none-blocking!",file.as_raw_fd());
        
    }

    fn register_sub_entry(&mut self, file : Arc<FileListenerEntry>){
        let entry = SlaveEntryInner{
            uuid : file.get_uuid(),
            inner_read : file.inner_read(),
            inner_write : file.inner_write(),
        };
        let slave_entry = match self.get_registered(&file.get_file()){
            Some(v) => v,
            None => {
                self.create_push_register_entry(file.get_file());
                match self.get_registered(&file.get_file()){
                    Some(v) => v,
                    None => panicerror!("Created file lost somehow!"),
                }
            },
        };
        slave_entry.entries.write().unwrap().push(Arc::new(entry));
        let _ = self.communication.send(FileListenerSignalTypes::SIGNALDONE);
    }

    fn unregister_sub_entry(&mut self, file : Arc<File>, uuid : Uuid){
        if let Some(entry) = self.get_registered(&file){
            if let Some(_inner) = entry.remove_entry(uuid){
                if entry.entries.write().unwrap().is_empty(){
                    self.epoll_file(entry, false, false);
                    //sdebug!("Removing entry {}!",file.as_raw_fd());
                    self.entries.remove_entry(&file.as_raw_fd());
                }
                ////sdebug!("Removed buffer with uuid {} for file {}",inner.uuid,file.as_raw_fd());
            }else{
                panicerror!("Called unregistered for file {} buffer {} which isn't registered!",file.as_raw_fd(), uuid)
            }
        }else {
            panicerror!("Called unregistered for file {} which isn't registered!",file.as_raw_fd())
        }
        let _ = self.communication.send(FileListenerSignalTypes::SIGNALDONE);
    }

    fn handle_queue(&mut self){
        loop{
            if let Some(to_register) = self.queue.pop(){
                match to_register.clone(){
                    FileListenerCommunication::REGISTERFILEENTRY { entry } => self.register_sub_entry(entry),
                    FileListenerCommunication::UNREGISTERFILE { file, uuid } => self.unregister_sub_entry(file, uuid),
                };
                
                break;
            }
        }

        //debug!("Registered file {}",file.get_raw_fd());
    }
    
    
    fn handle_command(&mut self) -> FileListenerSlaveState{
        let signal = self.communication.read::<FileListenerSignalTypes>().unwrap();
        
        match signal{
            FileListenerSignalTypes::SIGNALDONE => {panic!("Illegal signal received from master!")},
            FileListenerSignalTypes::FROMQUEUE => {
                self.handle_queue();
                FileListenerSlaveState::Listening
            },
            FileListenerSignalTypes::SLAVESTARTED => panic!("Illegal signal received from master, how to handle?"),
        }
    }

    fn set_eof(&self,entry : Arc<SlaveEntry>, mut can_write : bool){
        for entry in &*entry.entries.write().unwrap(){
            let read = entry.inner_read.clone();
            read.is_eof.store(true, std::sync::atomic::Ordering::Release);
            read.waker.wake();
        
            let write = entry.inner_write.clone();
            if !can_write || write.is_eof.load(std::sync::atomic::Ordering::Acquire){
                can_write = false;
                write.is_eof.store(true, std::sync::atomic::Ordering::Release);
                write.waker.wake();
            }
            
            
        }
        //self.epoll_file(entry, can_write, false);
    }
    
    fn listen_epoll(&mut self) -> FileListenerSlaveState{
        
        let mut next_state = FileListenerSlaveState::Listening;
        let read = self.epoll.wait(&mut self.epoll_buffer,PollTimeout::NONE).unwrap();
        let mut read_buffer =  vec![0_u8; Self::READ_BUFFER_SIZE];
        //sdebug!("Handling Listening event, read {} events!",read);
        for i in 0..read{
            let event = self.epoll_buffer[i];
            if event.data() == self.read_pipe_fd as u64{
            sdebug!("Received ReadCommand!");

                next_state = FileListenerSlaveState::ReadCommand;
                if event.events().contains(EpollFlags::EPOLLERR){
                    panicerror!("Command channel broken!");
                }
                continue;
            }
            if self.show_logs{
                sdebug!("[{}] {}",event.data(),epoll_events::get_events_string(&event).join(", "))
            }

            if event.events().contains(EpollFlags::EPOLLIN){
                self.epoll_in(event.data() as RawFd, &mut read_buffer);
                
            }
            if event.events().contains(EpollFlags::EPOLLOUT){
                //sdebug!("EPOLLOUT {}",event.data());
                if let Some(entry) = self.entries.get(&(event.data() as RawFd)){
                    let lock = entry.entries.write().unwrap();
                    for inner in &*lock{
                        let inner = inner.inner_write.clone();
                        inner.can_write.store(true, std::sync::atomic::Ordering::Release);
                        inner.waker.wake();
                    }
                    drop(lock);
                }else{
                    panicerror!("Could not find file!");
                }
            }
            if event.events().contains(EpollFlags::EPOLLHUP){
                let write_open = false;//event.events().contains(EpollFlags::EPOLLRDHUP) && !event.events().contains(EpollFlags::EPOLLHUP);
                //sdebug!("EHOLLUP OR EPOLLRDHUP {}, write open : {}", event.data(),write_open);

                if let Some(entry) = self.entries.get(&(event.data() as RawFd)){
                    self.set_eof(entry.clone(), write_open);
                }
                self.epoll_in(event.data() as RawFd, &mut read_buffer);
            }
            if event.events().contains(EpollFlags::EPOLLERR){
                    if event.events().contains(EpollFlags::EPOLLHUP){
                    serror!("ERROR FOR FILE BUT WITH EPOLLUP: {}",event.data());
                    continue;
                    }else{
                    let write_open = false;
                    serror!("EPOLLERR {}", event.data());

                    if let Some(entry) = self.entries.get(&(event.data() as RawFd)){
                        serror!("Found entry to delete!");
                        self.set_eof(entry.clone(), write_open);
                    }else{
                        panicerror!("Found no entry to delete!");
                    }
                    }
            }
        }
            //sdebug!("Finished with Epollevents!");
        next_state
    }

    fn epoll_in(&mut self, file : RawFd, read_buffer : &mut [u8]){
        if let Some(entry) = self.entries.get(&file){
            //sdebug!("Locking file {}",file);
            let entries = entry.entries.write().unwrap();
            let mut file = entry.file.clone();
            //sdebug!("Locked file {}",file.as_raw_fd());
            loop{
                match file.read(read_buffer){
                    Ok(0) =>{
                        drop(entries);
                        self.set_eof(entry.clone(),true);
                        break;
                    }
                    Ok(read) => {
                        for entry in &*entries{
                            ////sdebug!("Read {} bytes for {}!",read,entry.uuid);
                            
                            let inner = entry.inner_read.clone();
                            for &byte in read_buffer.iter().take(read) {
                                inner.buffer.push(byte);
                            }
                            inner.waker.wake();

                        }
                    },
                    Err(err) => {
                        if err.kind() != std::io::ErrorKind::WouldBlock{
                            serror!("Error received reading file : {}",err);
                            drop(entries);
                            self.set_eof(entry.clone(), false);

                            //sdebug!("Finished with eof!");
                        }else{
                            for entry in &*entries{
                                let inner = entry.inner_read.clone();
                                //debug!("Waking up {}",entry.uuid);
                                inner.waker.wake();
                            }                   
                        }
                        ////sdebug!("Read file {}",file.as_raw_fd());
                        break;
                    },
                }
            }
        }
    }

    fn run(&mut self){
        //debug!("Started FileListenerSlave!");
        self.communication.send(FileListenerSignalTypes::SLAVESTARTED).unwrap();
        
        loop{
            self.state = match self.state{
                FileListenerSlaveState::Listening => {
                    //sdebug!("Listening for epolls!");
                    self.listen_epoll()
                },
                FileListenerSlaveState::ReadCommand => {
                    //sdebug!("Listening for commands!");
                    self.handle_command()
                },
            }
        }
    }
}