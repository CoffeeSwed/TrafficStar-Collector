use std::collections::HashMap;

use reqwest::{
    Method, header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue}
};
use serde::{Deserialize, Serialize};
use trafficstar_logger::serror;
use trafficstar_logger_macro::StructLoggerName;

use crate::trafficstar_mullvad_device::MullvadDevice;

pub const MULLVAD_API_RELAYS: &str = "https://api.mullvad.net/public/relays/wireguard/v1/";
pub const MULLVAD_API_AUTH: &str = "https://api.mullvad.net/auth/v1/";
pub const MULLVAD_API_AUTH_TOKEN: &str = "token";
pub const MULLVAD_API_ACCOUNTS: &str = "https://api.mullvad.net/accounts/v1/";
pub const MULLVAD_API_ACCOUNTS_DEVICES: &str = "devices";
pub const PERMISSION_ERROR_CODE : u16 = 401;
#[derive(Serialize, Deserialize)]
pub struct CreateToken {
    pub account_number: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub access_token: String,
    pub expiry: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceRequest {
    pub pubkey: String,
    pub hijack_dns: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct MullvadDeviceResponse {
    pub id: String,
    pub name: String,
    pub pubkey: String,
    pub ipv4_address: String,
    pub ipv6_address: String,
    pub hijack_dns: bool,
    pub created: String,
}


impl From<MullvadDeviceResponse> for MullvadDevice {

    fn from(device: MullvadDeviceResponse) -> Self {
        Self {
            created: device.created,
            hijack_dns: device.hijack_dns,
            id: device.id,
            ipv4_address: device.ipv4_address,
            ipv6_address: device.ipv6_address,
            name: device.name,
            pubkey: device.pubkey,
            account_number : "Unknown".to_string()
        }
    }
}

#[derive(StructLoggerName)]
///Factory for creating mullvad requests.
pub struct MullvadRequestBuilder {
    url: String,
    method: reqwest::Method,
    headers: HeaderMap,
    query_params: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl MullvadRequestBuilder {
    
    pub fn new(url: String) -> Self {
        MullvadRequestBuilder {
            url,
            body: None,
            method: Method::GET,
            headers: HeaderMap::new(),
            query_params: HashMap::new(),
        }
    }

    // Method to set the HTTP method
    pub fn method(mut self, method: reqwest::Method) -> Self {
        self.method = method;
        self
    }

    // Method to add a header
    pub fn header(mut self, key: HeaderName, value: &str) -> Self {
        self.headers
            .insert(key, HeaderValue::from_str(value).unwrap());
        self
    }

    // Method to add query parameters
    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.query_params.insert(key.to_string(), value.to_string());
        self
    }

    // Method to set the request body
    pub fn body<S: serde::Serialize>(mut self, body: S) -> Self {
        if let Ok(vec) = serde_json::to_vec(&body) {
            self.body = Some(vec)
        } else {
            serror!(
                "Could not overwrite the body since failed to convert given Serializable to vec!"
            )
        }
        self
    }

    // Method to set request body to json
    pub fn json<T: Serialize>(self, json: &T) -> Self {
        self.body(json).header(CONTENT_TYPE, "application/json")
    }

    // Method to use a token
    pub fn use_token(self, token: Option<String>) -> Self {
        if let Some(token) = token {
            let bearer_value = format!("Bearer {}", token);
            return self.header(AUTHORIZATION, &bearer_value);
        }
        self
    }

    pub async fn send_async(self) -> Result<reqwest::Response, std::io::Error>{
        let mut request_builder = reqwest::Client::new().request(self.method.clone(), &self.url);

        // Add query parameters
        for (key, value) in self.query_params {
            request_builder = request_builder.query(&[(key, value)]);
        }

        // Add headers
        request_builder = request_builder.headers(self.headers);

        // Add body if present
        if let Some(body) = self.body {
            request_builder = request_builder.body(body);
        }

        // Send the request and get the response body
        match request_builder.send().await {
            Ok(response) =>{
                if response.status().as_u16() == PERMISSION_ERROR_CODE{
                    Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Bad token!"))
                }else{
                    Ok(response)
                }
            },
            Err(_err) => {
                Err(std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, "Network failure!"))
            },
        }
    }
}
