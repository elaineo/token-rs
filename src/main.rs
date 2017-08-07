extern crate serde;
extern crate serde_json;
extern crate gethrpc;
extern crate shapeshift;
extern crate emerald_core as emerald;

#[macro_use] 
extern crate nickel;
#[macro_use]
extern crate serde_derive;

extern crate bincode;
extern crate leveldb;
extern crate hyper;
extern crate time;
extern crate hex;

use bincode::deserialize;

use std::path::Path;
use std::sync::Arc;

use nickel::{Nickel, HttpRouter, Request, Response, MiddlewareResult};
use nickel::status::StatusCode;
use hyper::header::{AccessControlAllowOrigin, AccessControlAllowHeaders, AccessControlAllowMethods, ContentType};
use hyper::method::Method;

use std::thread;

mod raw_body;
mod token_db;

use raw_body::*;
use token_db::{TokenDB, ShapeshiftDeposit};

use gethrpc::{GethRPCClient};
use shapeshift::{ShapeshiftClient, ShapeshiftStatus};
use emerald::util::{to_arr, align_bytes};
use emerald::{Transaction, Address};
use hex::{FromHex, ToHex};

#[derive(Serialize, Deserialize)]
struct RPCResponse {
    result: String,
    id: usize,
}

const DEFAULT_DIR: &'static str = "./tokendb";

const ICO_MACHINE: &'static str = "0x2f846034a256f51ae51249b61f4c92bcf4b0a3d8";

// TODO: Generate new one for each tx
const RELAY_ACCOUNT: &'static str = "0xE4cf8aE5E9Cdc78d0d339877f05CD190Cc6f4d54";                             

fn enable_cors<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    res.set(AccessControlAllowOrigin::Any);

    res.set(AccessControlAllowMethods(vec![
        Method::Get,
        Method::Post,
        Method::Options,
        Method::Delete,
        ])
    );
    res.set(AccessControlAllowHeaders(vec![
    // Hyper uses the `unicase::Unicase` type to ensure comparisons are done
    // case-insensitively. Here, we use `into()` to convert to one from a `&str`
    // so that we don't have to import the type ourselves.
    "Origin".into(),
    "X-Requested-With".into(),
    "Content-Type".into(),
    "Accept".into(),
    ]));
    res.next_middleware()
}
fn enable_options_preflight<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    res.set(ContentType::plaintext());
    res.send("Ok")
}

fn to_32bytes(hex: &str) -> [u8; 32] {
    to_arr(Vec::from_hex(&hex).unwrap().as_slice())
}

fn main() {
    let mut server = Nickel::new();
    server.utilize(enable_cors);
    server.options("**/*", enable_options_preflight);

    let path = Path::new(DEFAULT_DIR);
    let db = Arc::new(TokenDB::new(path));
    let read_db = db.clone();
    let all_db = db.clone();
    let poll_db = db.clone();

    let client_addr = "https://mewapi.epool.io";
    
    let mut client = GethRPCClient::new(client_addr);
    let mut ss_client = ShapeshiftClient::new();

    // receive deposit, add to DB
    server.post("/add", middleware! { |req, res| 
        let raw = req.raw_body();
        let mut deposit = serde_json::from_str::<ShapeshiftDeposit>(&raw).unwrap();
        deposit.status = "no_deposits".to_string();
        db.write_deposit(&deposit);
        
        (StatusCode::Ok, "{\"result\": \"ok\"}")
    });

    server.get("/key/:id", middleware! { |req|
        let id = req.param("id").unwrap();
        println!("id: {}", id);
        let key: i32 = id.parse()
                    .expect("Failed to parse key");

        let data = read_db.read_deposit(key)
            .expect("Failed to lookup key");

        let deposit: ShapeshiftDeposit = deserialize(&data)
            .expect("Corrupted entry in db");

        match serde_json::to_string(&deposit) {
            Ok(res) => { (StatusCode::Ok, res.to_string()) },
            Err(e) => { (StatusCode::NotFound, e.to_string()) }
        }
    });

    server.get("/all", middleware! {
        let data = all_db.dump();
        let deposit: Vec<ShapeshiftDeposit> = data.iter().map(|x| deserialize(&x).unwrap()).collect();
        
        format!("{}", serde_json::to_string(&deposit).unwrap())
    });
   
    thread::spawn(|| {
        server.listen("127.0.0.1:8000");
    });

    loop {
        // if funded, verify and buy tokens
        
        let mut ss: ShapeshiftStatus;
        let timespec = time::get_time();
        let now = timespec.sec + timespec.nsec as i64 / 1000 / 1000;
        let data = poll_db.dump();
        let deposit_full: Vec<ShapeshiftDeposit> = data.iter().map(|x| deserialize(&x).unwrap()).collect();
        for d in 0..deposit_full.len() {
            if deposit_full[d].status == "complete".to_string() {
                continue;
            }
            ss = ss_client.get_status(&deposit_full[d].deposit);
            println!("Status of {:?}: {:?}", ss.address, ss.status);
            if ss.status == "complete".to_string() {
                let deposit_amount: [u8; 32] =  to_arr(&align_bytes(&deposit_full[d].depositAmount.as_bytes(), 32));
                let bal: String = client.get_balance(&ss.withdraw.unwrap(), "latest");
                let bal_amount: [u8; 32] = to_arr(&align_bytes(&bal.as_bytes(), 32));
                if bal_amount >= deposit_amount {
                    let gas_price = to_32bytes(&client.gas_price());
                    let nonce: u64 = client.get_transaction_count(RELAY_ACCOUNT, "latest")
                        .parse()
                        .expect("Failed to parse nonce");
                    let data = align_bytes(&deposit_full[d].address.as_bytes(), 64);
                    let tx = Transaction {
                        nonce: nonce,
                        gas_price: gas_price,
                        gas_limit: 21000,
                        to: Some(ICO_MACHINE                                
                            .parse::<Address>()
                            .unwrap()),
                        value: deposit_amount,
                        data: data,
                    };
                    println!("send {:?}", deposit_amount);
                    let mut d_complete = deposit_full[d].clone();
                    d_complete.status = "complete".to_string();
                    poll_db.write_deposit(&d_complete);
                }
            }
            // check expiration -- if expired, Delete
            else if now*1000 > deposit_full[d].expiration as i64 {
                poll_db.delete_deposit(deposit_full[d].id as i32);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(1_000));
    }

}
