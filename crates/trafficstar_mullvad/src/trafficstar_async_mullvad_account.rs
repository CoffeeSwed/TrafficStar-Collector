use std::sync::Arc;

use chrono::{DateTime, Utc};

use tokio::sync::RwLock;
use trafficstar_connections::trafficstar_wireguard::WireguardKeys;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{serror, sinfo};
use trafficstar_logger_macro::StructLoggerName;


use crate::{
    trafficstar_mullvad_device::MullvadDevice, trafficstar_mullvad_requests::{
        CreateToken, CreateTokenResponse, DeviceRequest, MULLVAD_API_ACCOUNTS,
        MULLVAD_API_ACCOUNTS_DEVICES, MullvadDeviceResponse, MullvadRequestBuilder,
    }, 
};

#[derive(Clone)]
pub struct MullvadHandlerAccountToken{
    token : String,
    expiry : DateTime<Utc>
}

#[derive(StructLoggerName)]
pub struct AsyncMullvadAccount {
    token: Arc<RwLock<Option<MullvadHandlerAccountToken>>>,
    account_number: String,
}

#[allow(dead_code, unsafe_code, static_mut_refs)]
impl AsyncMullvadAccount {
    pub async fn new(account_number: String) -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
            account_number,
        }
        
    }

    pub fn account_number(&self) -> &str{
        &self.account_number
    }
}


#[allow(dead_code, unsafe_code, static_mut_refs)]
impl AsyncMullvadAccount {
    pub async fn get_token(&self) -> Option<String> {
        if let token = self.token.read().await
            && let Some(token_memory) = (*token).clone(){
                let token_expiry = token_memory.expiry;
                let current_time: DateTime<Utc> = Utc::now();
                let duration = token_expiry.signed_duration_since(current_time);
                if current_time > token_expiry {
                    sinfo!("Current token needs to be refreshed since it has expired!");
                } else if duration.num_minutes() <= 5 {
                    sinfo!("Current token will expire in five minutes, refreshing token!");
                }else{
                    return Some(token_memory.token)
                }
        }
        let mut token_memory = self.token.write().await;
        sinfo!("Fetching token!");
        let request_data = CreateToken {
            account_number: self.account_number.clone(),
        };
        let client = reqwest::Client::new();
        let response = client
            .post(
                crate::trafficstar_mullvad_requests::MULLVAD_API_AUTH.to_string()
                    + crate::trafficstar_mullvad_requests::MULLVAD_API_AUTH_TOKEN,
            )
            .json(&request_data)
            .send().await;
        match response {
            Err(v) => {
                serror!("Received error {} when trying to generate a token!", v);
            }
            Ok(v) => {
                if let Ok(json) = v.json::<CreateTokenResponse>().await {
                    if let Ok(expiry) = DateTime::parse_from_rfc3339(&json.expiry) {
                        let to_return = json.access_token.clone();
                        *token_memory = Some(MullvadHandlerAccountToken{
                            token: json.access_token,
                            expiry: expiry.with_timezone(&Utc),
                        });
                        sinfo!("Token fetched from api!");
                        return Some(to_return);
                    } else {
                        serror!("Expiry time given is not in the expected format!")
                    }
                } else {
                    serror!(
                        "Didn't receive expected json response from api when generating a token!"
                    );
                }
            }
        }
    
        None
    }

    //Fetch devices from API
    pub async fn fetch_devices(&self) -> Result<Vec<MullvadDevice>, TrafficStarError>{
        let response = MullvadRequestBuilder::new(
            MULLVAD_API_ACCOUNTS.to_string() + MULLVAD_API_ACCOUNTS_DEVICES,
        )
        .use_token(self.get_token().await)
        .send_async().await?;
      
        let mut result: Vec<MullvadDevice> = Vec::new();
        let devices: Vec<MullvadDeviceResponse> = match response.json().await{
            Ok(v) => v,
            Err(err) => return Err(format!("{}",err).into()),
        };
        for device in devices {
            let mut v = MullvadDevice::from(device);
            v.account_number = self.account_number.clone();
            result.push(v)
        }
        Ok(result)
           
        }
    

    pub async fn push_delete_device(&self, device: &MullvadDevice) -> Result<reqwest::Response, TrafficStarError> {
        let response = MullvadRequestBuilder::new(
            MULLVAD_API_ACCOUNTS.to_string() + MULLVAD_API_ACCOUNTS_DEVICES + "/" + &device.id,
        )
        .use_token(self.get_token().await)
        .method(reqwest::Method::DELETE)
        .send_async().await?;
        Ok(response)
    }


    pub async fn push_create_device_request(&self, keys: &WireguardKeys) -> Result<MullvadDevice,TrafficStarError> {
        let res = MullvadRequestBuilder::new(
            MULLVAD_API_ACCOUNTS.to_string() + MULLVAD_API_ACCOUNTS_DEVICES,
        )
        .use_token(self.get_token().await)
        .method(reqwest::Method::POST)
        .json(&DeviceRequest {
            hijack_dns: false,
            pubkey: keys.pubkey.to_base64(),
        })
        .send_async().await?;
        match res.json::<MullvadDeviceResponse>().await{
            Ok(v) => Ok(MullvadDevice::from(v)),
            Err(err) => Err(format!("{}",err).into()),
        }
    }

}
