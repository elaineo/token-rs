extern crate serde;
extern crate serde_json;
extern crate gethrpc;
extern crate shapeshift;

#[macro_use] 
extern crate nickel;
#[macro_use]
extern crate serde_derive;

extern crate bincode;
extern crate leveldb;
extern crate hyper;

use bincode::deserialize;

use std::path::Path;
use std::sync::Arc;

use nickel::{Nickel, HttpRouter, Request, Response, MiddlewareResult, MediaType};
use nickel::status::StatusCode;
use hyper::header::{AccessControlAllowOrigin, AccessControlAllowHeaders,AccessControlAllowMethods, ContentType};
use hyper::method::Method;

use std::thread;
use std::time::Duration;

mod raw_body;
mod token_db;

use raw_body::*;
use token_db::{TokenDB, ShapeshiftDeposit};

use gethrpc::{GethRPCClient};
use shapeshift::{ShapeshiftClient, ShapeshiftStatus};

#[derive(Serialize, Deserialize)]
struct RPCResponse {
    result: String,
    id: usize,
}

const DEFAULT_DIR: &'static str = "./tokendb";

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
    // Set HTTP 200.
    // ContentType should be text/plain.
    res.set(ContentType::plaintext());
    // Just "ok" as response text.
    res.send("Ok")
}

fn main() {
    let mut server = Nickel::new();
    server.utilize(enable_cors);
    server.options("**/*", enable_options_preflight);

    let path = Path::new(DEFAULT_DIR);
    let db = Arc::new(TokenDB::new(path));
    let read_db = db.clone();
    let all_db = db.clone();
    let addr_db = db.clone();

    let client_addr = "https://mewapi.epool.io";
    
    let mut client = GethRPCClient::new(client_addr);
    let mut ss_client = ShapeshiftClient::new();

    let x: String = client.client_version();

    // receive deposit, add to DB
    server.post("/add", middleware! { |req, res| 
        let raw = req.raw_body();
        let deposit = serde_json::from_str::<ShapeshiftDeposit>(&raw).unwrap();
        db.write_deposit(&deposit);
        
        println!("Deposit Received {}", deposit.address);
        
        //let json_obj = json::encode(&v).unwrap();
        //res.set(MediaType::Json);
        //res.set(StatusCode::Ok);
        //return res.send(json_obj);
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
        //poll address status
        // check expiration -- if expired, Delete
        // if funded, buy tokens
        
        let addrs = addr_db.dump_addrs();
        let mut ss: ShapeshiftStatus;
        for a in 0..addrs.len() {
            ss = ss_client.get_status(&addrs[a]);    
            println!("Status of {:?}: {:?}", ss.address, ss.status);
        }

        std::thread::sleep(std::time::Duration::from_millis(1_000));
    }

}
