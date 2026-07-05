use std::{io::{Error, pipe}, os::fd::OwnedFd, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use trafficstar_errors::traffic_star_error::TrafficStarError;

use crate::{trafficstar_files::file_handler::FileHandler, trafficstar_pipes::{pipe_async_receiver::AsyncPipeReceiver, pipe_async_writer::AsyncPipeSender, pipe_receiver::PipeReceiver, pipe_sender::PipeSender}};

pub mod pipe_receiver;
pub mod pipe_sender;
pub mod pipe_error;
pub mod pipe_async_receiver;
pub mod pipe_async_writer;

pub fn create_pipes() -> Result<(PipeSender, PipeReceiver), Error>{
    match pipe(){
        Ok(pipes) => {
            let writer : OwnedFd = pipes.1.into();
            let reader : OwnedFd = pipes.0.into();
            Ok((PipeSender::new(Arc::new(writer.into()))?, PipeReceiver::new(Arc::new(reader.into()))?))
        },
        Err(err) => Err(err),
    }
}

pub struct TrafficStarPipePair{
    pub output : PipeSender,
    pub input : PipeReceiver,
}


impl TrafficStarPipePair{
    pub fn new_pairs() -> Result<(TrafficStarPipePair,TrafficStarPipePair), TrafficStarError>{
        let (out_one, in_two) = create_pipes()?;
        let (out_two, in_one) = create_pipes()?;
        Ok((
            TrafficStarPipePair{
                input : in_one,
                output : out_one,
            },
            TrafficStarPipePair{
                input : in_two,
                output : out_two,
            }
        ))

    }

    pub fn read<T : DeserializeOwned>(&mut self) -> Result<T, TrafficStarError> {
        self.input.read()
    }

    pub fn send<T : Serialize>(&mut self, data : T) -> Result<(), TrafficStarError> {
        self.output.send(data)
    }

   
}
///If cloned, the AsyncPipeReceiver will be shared. To get one where it has it's own ReadBuffer, use new_receiver_clone
#[derive(Clone)]
pub struct TrafficStarPipePairAsync{
    pub output :  Arc<Mutex<AsyncPipeSender>>,
    pub input : Arc<Mutex<AsyncPipeReceiver>>,
}
impl TrafficStarPipePairAsync{
    pub async fn new_pairs() -> Result<(TrafficStarPipePairAsync,TrafficStarPipePairAsync), TrafficStarError>{
        let (in_two, out_one) = pipe()?;
        let (in_one, out_two) = pipe()?;
        Ok((
            TrafficStarPipePairAsync{
                input : Arc::new(Mutex::new(AsyncPipeReceiver::new(FileHandler::new(in_one.into()).await?)?)),
                output : Arc::new(Mutex::new(AsyncPipeSender::new(FileHandler::new(out_one.into()).await?)?)),
            },
            TrafficStarPipePairAsync{
                input : Arc::new(Mutex::new(AsyncPipeReceiver::new(FileHandler::new(in_two.into()).await?)?)),
                output : Arc::new(Mutex::new(AsyncPipeSender::new(FileHandler::new(out_two.into()).await?)?)),
            }
        ))
    }

    pub async  fn read<T : DeserializeOwned>(&self) -> Result<T, TrafficStarError> {
        self.input.clone().lock().await.read().await
    }

    pub async  fn send<T : Serialize>(&self, data : T) -> Result<(), TrafficStarError> {
        self.output.clone().lock().await.send(data).await
    }

    pub async fn try_from(value : TrafficStarPipePair) -> Result<Self, TrafficStarError>{
        let input = AsyncPipeReceiver::new(FileHandler::from_file(value.input.take()).await)?;
        let output = AsyncPipeSender::new(FileHandler::from_file(value.output.take()).await)?;
        Ok(Self{
            input : Arc::new(Mutex::new(input)),
            output : Arc::new(Mutex::new(output)),
        })
    }
}

