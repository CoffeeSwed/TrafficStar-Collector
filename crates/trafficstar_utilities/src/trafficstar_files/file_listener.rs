use std::{fs::File, io::Write, os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd}, sync::{Arc, Mutex, OnceLock, atomic::AtomicBool}, task::Poll};

use trafficstar_logger::{panicerror, sdebug, trafficstar_logger::TrafficStarLogger};
use trafficstar_logger_macro::StructLoggerName;
use trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName;
use uuid::Uuid;
use futures::task::AtomicWaker;

use crossbeam_queue::SegQueue;
use crate::{trafficstar_files::{file_listener_slave::FileListenerSlave, file_signals::{FileListenerCommunication, FileListenerSignalTypes}}, trafficstar_pipes::TrafficStarPipePair};


pub struct FileListenerEntryReadBuffer{
    pub buffer : SegQueue<u8>,
    pub is_eof : AtomicBool,
    pub waker : AtomicWaker
}

pub struct FileListenerEntryWriteWrapper{
    pub can_write : AtomicBool,
    pub is_eof : AtomicBool,
    pub waker : AtomicWaker,
}



impl Default for FileListenerEntryReadBuffer{
    fn default() -> Self {
        Self { buffer: SegQueue::new(), is_eof: AtomicBool::new(false), waker : AtomicWaker::new() }
    }
}

impl Default for FileListenerEntryWriteWrapper{
    fn default() -> Self {
        Self { can_write : AtomicBool::new(true), is_eof: AtomicBool::new(false), waker : AtomicWaker::new()}
    }
}


#[derive(StructLoggerName)]
pub struct FileListenerEntry{
    fd : RawFd,
    file : Arc<File>,
    uuid : Arc<Uuid>,

    inner_read : Arc<FileListenerEntryReadBuffer>,

    inner_write : Arc<FileListenerEntryWriteWrapper>,
}

impl FileListenerEntry{
     pub fn borrow(&self) -> BorrowedFd<'_>{
        self.file.as_fd()
    }

    pub fn get_uuid(&self) -> Arc<Uuid>{
        self.uuid.clone()
    }

    pub fn try_clone_file(&self) -> Result<File, std::io::Error>{
        self.file.try_clone()
    }

    pub fn get_file(&self) -> Arc<File>{
        self.file.clone()
    }

    pub fn get_raw_fd(&self) -> RawFd{
        self.fd
    }

    pub fn is_read_eof(&self) -> bool{
        self.inner_read.is_eof.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn can_write(&self) -> bool{
        self.inner_write.can_write.load(std::sync::atomic::Ordering::Acquire) || self.inner_write.is_eof.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn inner_read(&self) -> Arc<FileListenerEntryReadBuffer>{
        self.inner_read.clone()
    }

    pub fn inner_write(&self) -> Arc<FileListenerEntryWriteWrapper>{
        self.inner_write.clone()
    }

    pub fn is_write_eof(&self) -> bool{
        self.inner_write.is_eof.load(std::sync::atomic::Ordering::Acquire)
    }

    /*
    1 check state
    2 register waker
    3 check state again
    4 Pending
     */

    pub fn poll_available_read(&self) -> usize{
        let inner = self.inner_read.clone();
        inner.buffer.len()
    }

    pub fn poll_read_internal(
        self: std::pin::Pin<Arc<FileListenerEntry>>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let inner = self.inner_read.clone();

        // Try reading data from the buffer
        let mut n = 0;
        while n < buf.len() {
            match inner.buffer.pop() {
                Some(v) => {
                    buf[n] = v;
                    n += 1;
                },
                None => break,
            }
        }

        // If we've read some data, return it
        if n > 0 {
            //sdebug!("Read file {}",self.fd);
            return std::task::Poll::Ready(Ok(n));
        }

        // If EOF is set and buffer is empty, return 0 bytes read
        if self.is_read_eof() {
            inner.waker.wake();
            return Poll::Ready(Ok(0));  // EOF reached, no more data available
        }

        // Register the waker if there's nothing to read and we haven't reached EOF
        inner.waker.register(cx.waker());

        let mut n = 0;
        while n < buf.len() {
            match inner.buffer.pop() {
                Some(v) => {
                    buf[n] = v;
                    n += 1;
                },
                None => break,
            }
        }

        // If we've read some data, return it
        if n > 0 {
            //sdebug!("Read file {} after register!",self.fd);
            return std::task::Poll::Ready(Ok(n));
        }


        // If buffer is empty and EOF is not set, return Pending for future data
        //sdebug!("Pending on file {}",self.fd);
        Poll::Pending
    }


      /*
        1 check state or Write
        2 register waker
        3 check state again, write or Pending
        4 Pending
     */

   pub fn poll_write_internal(
    self: std::pin::Pin<Arc<FileListenerEntry>>,
    cx: &mut std::task::Context<'_>,
    buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {

        let inner = self.inner_write.clone();

        if !self.can_write() {
            inner.waker.register(cx.waker());

            if !self.can_write() {
                return Poll::Pending;
            }
        }
        if inner.is_eof.load(std::sync::atomic::Ordering::Acquire){
            return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "File closed!")));
        }

        let mut n = 0;
        let mut file = self.get_file();
        while n < buf.len() {
            match file.write(&buf[n..]) {
                Ok(0) => {
                    

                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "write returned 0",
                    )));
                }

                Ok(v) => n += v,

                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    inner.can_write.store(false, std::sync::atomic::Ordering::Release);
                    if n > 0 {
                        break;
                    } else {
                        inner.waker.register(cx.waker());
                        if !self.can_write(){
                            return Poll::Pending;
                        }
                    }
                }

                Err(err) => {
                    inner.is_eof.store(true, std::sync::atomic::Ordering::Release);
                    inner.waker.wake();
                    return Poll::Ready(Err(err))
                },
            }
        }

        Poll::Ready(Ok(n))
    }

     /*
        1 check state or Flush
        2 register waker
        3 check state again, Flush or Pending
        4 Pending
     */

    pub fn poll_flush_internal(
        self: std::pin::Pin<Arc<FileListenerEntry>>,
        cx: &mut std::task::Context<'_>
    ) -> std::task::Poll<std::io::Result<()>> {
        let inner = self.inner_write.clone();

        if !self.can_write(){
            inner.waker.register(cx.waker());
            if !self.can_write(){
                return Poll::Pending
            }
        }
        match self.get_file().flush() {
                Ok(_) => {
                    Poll::Ready(Ok(()))
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    //inner.can_write.store(false, std::sync::atomic::Ordering::Release);
                    inner.waker.register(cx.waker());
                    Poll::Pending
                }
                Err(err) =>{
                    inner.can_write.store(true, std::sync::atomic::Ordering::Release);
                    inner.waker.wake();
                    Poll::Ready(Err(err))},
            }
        
    }

    pub fn poll_close_internal(
        self: std::pin::Pin<Arc<FileListenerEntry>>,
        _cx: &mut std::task::Context<'_>
    ) -> std::task::Poll<std::io::Result<()>> {
        /*let inner = self.inner_write.clone();
        inner.is_eof.store(true, std::sync::atomic::Ordering::Release);
        inner.waker.wake();
        let inner = self.inner_read.clone();
        inner.is_eof.store(true, std::sync::atomic::Ordering::Release);
        inner.waker.wake();
        */
        std::task::Poll::Ready(Ok(()))
    }
}




impl Clone for FileListenerEntry{
    fn clone(&self) -> Self {
        Self { fd : self.fd, 
            file: self.file.clone(), 
            inner_read: self.inner_read.clone(), 
            uuid : self.uuid.clone(), 
            inner_write : self.inner_write.clone(),
        }
    }
}

impl Drop for FileListenerEntry{
    fn drop(&mut self) {
        FileListenerMaster::get_singleton().drop_buffer(self.get_file(), *self.uuid); 
    }
}

#[derive(StructLoggerName)]
pub struct FileListenerMaster{
    communication : Arc<Mutex<TrafficStarPipePair>>,
    created_file_listeners : Arc<SegQueue<FileListenerCommunication>>
}


impl FileListenerMaster{
    fn new() -> Self{
        let (our_pair, their_pair) = TrafficStarPipePair::new_pairs().unwrap();
        let res = Self { communication: Arc::new(Mutex::new(our_pair)), created_file_listeners : Arc::new(SegQueue::new()) };
        FileListenerSlave::start( their_pair, res.created_file_listeners.clone());
        sdebug!("Waiting for slave to signal its ready!");
        let communication = res.communication.clone();
        let mut lock = communication.lock().unwrap();
        sdebug!("Got lock!");
        if lock.read::<FileListenerSignalTypes>().unwrap() != FileListenerSignalTypes::SLAVESTARTED{
            panic!("Unexpected signal, state machine incorrect?");
        }
        TrafficStarLogger::mute(Self::struct_name().into(), log::Level::Debug);
        sdebug!("Started FileListenerMaster!");
        res
    }



    #[allow(static_mut_refs)]
    ///Static behaviour required by logger.
    pub fn get_singleton() -> &'static FileListenerMaster {
        static SINGLETON: OnceLock<FileListenerMaster> = OnceLock::new();

        SINGLETON
            .get_or_init(FileListenerMaster::new)
    }
    
    ///Creates a new Clone but with its own read Buffer!
    pub fn new_read_clone(&self, entry : Arc<FileListenerEntry>) -> Arc<FileListenerEntry>{
        let entry = Arc::new(FileListenerEntry{
            fd : entry.file.as_raw_fd(),
            file: entry.get_file(),
            inner_read: Arc::new(FileListenerEntryReadBuffer::default()),
            uuid : Arc::new(uuid::Uuid::new_v4()),
            inner_write : Arc::new(FileListenerEntryWriteWrapper::default()),
        });
        self.register_file_listener(entry.clone());
        entry
    }
    
    pub fn register_file_listener(&self, entry : Arc<FileListenerEntry>){
        sdebug!("Register file {}!",entry.fd);
        let mut lock = self.communication.lock().unwrap();
        sdebug!("Holding lock file {}!",entry.fd);
        self.created_file_listeners.push(FileListenerCommunication::REGISTERFILEENTRY { entry: entry.clone() });
        lock.send(FileListenerSignalTypes::FROMQUEUE).unwrap();
        if lock.read::<FileListenerSignalTypes>().unwrap() != FileListenerSignalTypes::SIGNALDONE{
            panicerror!("Unexpected signal, state machine incorrect?");
        }
        sdebug!("Finish registering file {}!",entry.fd);

    }

    ///Must call unregister if not using FileHolder, otherwise unknown state can occur! Automatically done by FileHolder.
    pub fn register_file(&self, file : Arc<File>) -> Arc<FileListenerEntry>{
        let entry = Arc::new(FileListenerEntry{
            fd : file.as_raw_fd(),
            file,
            inner_read: Arc::new(FileListenerEntryReadBuffer::default()),
            uuid : Arc::new(uuid::Uuid::new_v4()),
            inner_write : Arc::new(FileListenerEntryWriteWrapper::default()),
        });
        self.register_file_listener(entry.clone());
        entry.clone()
    }
    

    fn drop_buffer(&self, file : Arc<File>, uuid : Uuid){
        sdebug!("Calling to drop regristered file {}!",file.as_raw_fd());
        let file_id = file.as_raw_fd();
        let mut lock = self.communication.lock().unwrap();
        sdebug!("Received lock {}!",file_id);
        self.created_file_listeners.push(FileListenerCommunication::UNREGISTERFILE { file, uuid });
        lock.send(FileListenerSignalTypes::FROMQUEUE).unwrap();
        sdebug!("Sent Message {}!",file_id);
        if lock.read::<FileListenerSignalTypes>().unwrap() != FileListenerSignalTypes::SIGNALDONE{
           panicerror!("Unexpected signal, state machine incorrect?");
        }
        sdebug!("Finished call to drop regristered file {}!",file_id);
    }
}