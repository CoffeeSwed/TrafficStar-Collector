#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct SinkSenderSettings{
    pub time : u32,
}
impl std::fmt::Display for SinkSenderSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();

        res += "SinkSenderSettings{time : ";
        res += &self.time.to_string();

        res += "}";

        write!(f, "{}", res)
    }
}