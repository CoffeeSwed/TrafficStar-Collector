
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
#[derive(EnumIter, Clone, Copy, strum_macros::Display, Deserialize, Serialize, PartialEq)]
pub enum HandshakeType {
    ///AddInterfaceType communicates a interface/medium present on the sender side to the receiver side.
    ReportConnectionTypes,
    TimeSynchronization,
    SwitchToDataMode,
    RunSink,
    RunProxy,
    ReportDedicatedDataChannelProxy,
    ReportDedicatedDataChannelSink,
    ReportTestDirectoryPrefix,
    ReportTestFileNamePrefix,
    DeleteOldTestIfExists,
    SaveTest,
    SetNickParts,
    Unknown,
    ReportRouteInfo,
}

#[derive(EnumIter, Clone, Copy, strum_macros::Display, Deserialize, Serialize, PartialEq)]
pub enum ProxyHandShakeType {
   StartPcap,
   Finished,
   NextTest,
   Stop,
   ReportWebsite,
   PreventSimultanousRecordings
}


#[derive(EnumIter, Clone, Copy, strum_macros::Display, Deserialize, Serialize, PartialEq)]
pub enum SinkHandShakeType {
   StartPcap,
   Finished,
   Stop,
   PreventSimultanousRecordings
}

