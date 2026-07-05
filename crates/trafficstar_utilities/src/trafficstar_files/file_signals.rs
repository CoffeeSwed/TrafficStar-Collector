use std::{fs::File, sync::Arc};

use uuid::Uuid;

use crate::trafficstar_files::file_listener::FileListenerEntry;




#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, strum_macros::Display)]
pub enum FileListenerSignalTypes{
    SIGNALDONE,
    SLAVESTARTED,
    FROMQUEUE
}

#[derive(Clone, strum_macros::Display)]
pub enum FileListenerCommunication{
    REGISTERFILEENTRY{
        entry : Arc<FileListenerEntry>
    },
    UNREGISTERFILE{
        file : Arc<File>,
        uuid : Uuid
    }
}