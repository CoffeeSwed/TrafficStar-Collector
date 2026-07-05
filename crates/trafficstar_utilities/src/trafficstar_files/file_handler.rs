use std::{fs::File, os::fd::{BorrowedFd, OwnedFd, RawFd}, pin::Pin, sync::Arc};


use futures::AsyncReadExt;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use uuid::Uuid;

use crate::{trafficstar_files::file_listener::{FileListenerEntry, FileListenerMaster}};

///Creates a new File Holder from a OwnedFD with its own Read Buffer, created clones by clone() will have the same read buffer! Will set none-blocking.
#[derive(Clone)]
pub struct FileHandler{
    file_entry : Arc<FileListenerEntry>
}

impl FileHandler{
    ///Creates a new File Holder from a OwnedFD with its own Read Buffer, created clones by clones() will have the same read buffer! Will set none-blocking.
    pub async fn new(fd : OwnedFd) -> Result<FileHandler, TrafficStarError>{
        Ok(
        tokio::task::spawn_blocking(move || {
        FileHandler { 
            file_entry : FileListenerMaster::get_singleton().register_file(Arc::new(File::from(fd))),
        }}).await.unwrap()
        )
    }

    pub async fn from_file(file : Arc<File>) -> FileHandler{
        tokio::task::spawn_blocking(move || {
            FileHandler { file_entry: FileListenerMaster::get_singleton().register_file(file) }
        }).await.unwrap()
    }

    pub fn borrow(&self) -> BorrowedFd<'_>{
        
        self.file_entry.borrow()
    }

    ///Creates a new File Clone with its own Read Buffer, write is always shared. 
    pub async fn file_clone(&self) -> Result<FileHandler, TrafficStarError>{
        let file : OwnedFd = self.file_entry.try_clone_file()?.into();
        Self::new(file).await
    }

    ///Creates a new Clone but with its own Buffer!
    pub async fn read_clone(&self) -> FileHandler{
        let file_entry = self.file_entry.clone();
        tokio::task::spawn_blocking(move || {
        FileHandler { 
            file_entry : FileListenerMaster::get_singleton().new_read_clone(file_entry)
        }}).await.unwrap()
    }

    pub fn get_file(&self) -> Arc<File>{
        self.file_entry.get_file()
    }

    pub fn get_uuid(&self) -> Uuid{
        *self.file_entry.get_uuid()
    }

    pub fn raw_fd(&self) -> RawFd{
        self.file_entry.get_raw_fd()
    }

    pub fn take(self) -> Arc<File>{
        self.file_entry.get_file()
    }

    ///Reads a string from the buffer and returns it, vec is a sacrifical vector.
    /// Default stop_char is null character (0), if it reads EOF, it returns the sofar read string or an error if its the first character read!
    /// The stop_char is included unless EOF read.
    pub async fn read_string(&mut self, 
        mut vec : Vec<u8>, 
        stop_char : Option<u8>) -> Result<String, TrafficStarError>{
        if !vec.is_empty(){
            vec.clear();
        }
        let stop_char = stop_char.unwrap_or(0);
        let mut buf = [1_u8];
        if buf[0] == stop_char{
            if stop_char != u8::MAX{
                buf[0] = stop_char+1;
            }else{
                buf[0] = stop_char-1;
            }
        }
        
        while buf[0] != stop_char {

            if self.read(&mut buf).await? != 0{
               let char = buf[0];
               vec.push(char);
            }else {
                if vec.is_empty(){
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Received unexpected EOF").into())
                }
                break;
            }
        }
        match String::from_utf8(vec){
            Ok(v) => Ok(v),
            Err(err) => {
                Err(TrafficStarError::id_msg("FromUtf8Error".into(), format!("{}",err)))
            },
        }
        
    }


    pub fn available_read(&self) -> usize{
        self.file_entry.poll_available_read()
    }
    
}

impl futures::AsyncRead for FileHandler{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let k = Pin::new( this.file_entry.clone());
        k.poll_read_internal(cx, buf)
    }
}

impl futures::AsyncWrite for FileHandler{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
         let this = self.get_mut();
        let k = Pin::new( this.file_entry.clone());
        k.poll_write_internal(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let k = Pin::new( this.file_entry.clone());
        k.poll_flush_internal(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let k = Pin::new( this.file_entry.clone());
        k.poll_close_internal(cx)
    }
}

#[allow(unsafe_code)]
impl tokio::io::AsyncRead for FileHandler{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let k = Pin::new( this.file_entry.clone());
        let mut buffer = vec![0_u8;buf.remaining()];
        match k.poll_read_internal(cx, &mut buffer){
            std::task::Poll::Ready(r) => {
                match r{
                    Ok(v) => {
                        /*
                        SAFETY: We set the len to how much is read, so it's safe
                        */
                        unsafe { buffer.set_len(v) };
                        
                        buf.put_slice(&buffer);
                        std::task::Poll::Ready(Ok(()))

                    },
                    Err(r) => std::task::Poll::Ready(Err(r))

                }
                
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }

    }
}

impl Drop for FileHandler{
    fn drop(&mut self) {
        let inner = self.file_entry.clone();
        std::thread::spawn(move || {
            drop(inner);
        });
    }
}