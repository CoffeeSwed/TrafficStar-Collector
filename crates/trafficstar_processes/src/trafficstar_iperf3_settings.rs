#[derive(Clone)]
pub struct Iperf3Settings{
    pub report_interval : f64,
    pub connect_timeout_ms : u64,
    pub send_timeout_ms : u64,
    pub recv_timeout_ms : u64,

    pub idle_timeout_s : u64,
}

impl Default for Iperf3Settings{
    fn default() -> Self {
        Self { report_interval: 1.0, connect_timeout_ms : 10*1000, send_timeout_ms : 10*1000, recv_timeout_ms : 10*1000, idle_timeout_s : 10 }
    }
}