use trafficstar_utilities::sink::settings::SinkSenderSettings;

///dest_address is reported from the server.
#[derive(Clone)]
pub struct Iperf3ClientParams{
    pub dest_address : String,
    pub bind : Option<String>,
    pub iperf3settings : SinkSenderSettings
}

impl std::fmt::Display for Iperf3ClientParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();

        res += "Iperf3ClientParams{dest_address : ";
        res += &self.dest_address;

        res += ", bind : ";
        match &self.bind {
            Some(v) => res += v,
            None => res += "None",
        }


        res += ", iperf3settings : ";
        
        res += &format!("{}",self.iperf3settings.clone());
            

        res += "}";

        write!(f, "{}", res)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Iperf3ClientError {
    #[error("To many packets were dropped for a report interval.")]
    DroppedPackets,
    #[error("Couldn't connect within timeout period.")]
    FailedToConnect,
    #[error("Failed to read report.")]
    FailedToReadReport,
    #[error("Unknown error, did not determine what happend.")]
    Unknown
}

