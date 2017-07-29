extern crate serde;
extern crate serde_json;
extern crate hyper;
extern crate hyper_native_tls;

use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;

#[macro_use]
extern crate serde_derive;

use std::io::Read;

#[derive(Serialize, Deserialize)]
struct RPCRequest {
    jsonrpc: String,
    method: String,
    params: Vec<String>,
    id: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct RPCObjectRequest<T> {
    jsonrpc: String,
    method: String,
    params: T,
    id: usize,
}

#[derive(Serialize, Deserialize)]
struct ShapeshiftObjectResponse<T> {
    jsonrpc: String,
    result: T,
    id: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShapeshiftStatus {
    pub status: String,
    pub address: String,
    pub withdraw: Option<String>,     //[withdrawal address],
    pub incomingCoin: Option<String>, //[amount deposited],
    pub incomingType: Option<String>, //[coin type of deposit],
    pub outgoingCoin: Option<String>, //[amount sent to withdrawal address],
    pub outgoingType: Option<String>, //[coin type of withdrawal],
    pub transaction: Option<String>,  //[transaction id of coin sent to withdrawal address],
    pub error: Option<String>,
}

pub struct ShapeshiftClient {
    free_id: usize,
    http: Client,
}

impl ShapeshiftClient {
    pub fn new() -> Self {
        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        ShapeshiftClient {
            free_id: 1,
            http: Client::with_connector(connector),
        }
    }

    fn rpc_request<T: serde::Deserialize>(&mut self, method: &str, param: &str) -> ShapeshiftStatus {
        self.rpc_object_request::<ShapeshiftStatus>(method, param)
    }

    fn rpc_object_request<Res: serde::Deserialize>(&mut self, method: &str, param: &str) -> ShapeshiftStatus {
        let mut endpoint = "https://shapeshift.io/".to_string();
        endpoint.push_str(&method);
        endpoint.push_str(&"/".to_string());
        endpoint.push_str(&param);

        self.free_id = self.free_id + 1;

        let mut response_raw = self.http.get(&endpoint)
            .send().unwrap();

        let mut buffer = String::new();
        response_raw.read_to_string(&mut buffer).unwrap();

        let response: ShapeshiftStatus = serde_json::from_str(&buffer).unwrap();
        response

    }

    pub fn get_status(&mut self, address: &str) -> ShapeshiftStatus {
        self.rpc_request::<ShapeshiftStatus>("txStat", &address.to_string())
    }

    pub fn get_time_remaining(&mut self, address: &str) -> ShapeshiftStatus {
        self.rpc_request::<ShapeshiftStatus>("timeremaining", &address.to_string())
    }

}
