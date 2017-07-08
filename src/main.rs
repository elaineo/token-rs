extern crate serde;
extern crate serde_json;

#[macro_use] 
extern crate nickel;
#[macro_use]
extern crate serde_derive;

extern crate bincode;
extern crate leveldb;

use bincode::deserialize;

use std::path::Path;
use std::sync::Arc;

use nickel::{Nickel, HttpRouter};
use nickel::status::StatusCode;

mod raw_body;
mod token_db;

use raw_body::*;
use token_db::{TokenDB, SSDeposit};

const DEFAULT_DIR: &'static str = "./tokendb";

fn main() {
    let mut server = Nickel::new();

    let path = Path::new(DEFAULT_DIR);
    let db = Arc::new(TokenDB::new(path));
    let read_db = db.clone();
    let all_db = db.clone();

    server.post("/", middleware! { |req, res| 
        let raw = req.raw_body();
        let deposit = serde_json::from_str::<SSDeposit>(&raw).unwrap();
        db.write_deposit(&deposit);
        
        format!("Deposit Received {} {}", deposit.address, deposit.status)
    });

    server.get("/key/:id", middleware! { |req|
        let id = req.param("id").unwrap();
        println!("id: {}", id);
        let key: i32 = id.parse()
                    .expect("Failed to parse key");

        let data = read_db.read_deposit(key)
            .expect("Failed to lookup key");

        let deposit: SSDeposit = deserialize(&data)
            .expect("Corrupted entry in db");

        match serde_json::to_string(&deposit) {
            Ok(res) => { (StatusCode::Ok, res.to_string()) },
            Err(e) => { (StatusCode::NotFound, e.to_string()) }
        }
    });

    server.get("/all", middleware! {
        let data = all_db.dump();
        let deposit: Vec<SSDeposit> = data.iter().map(|x| deserialize(&x).unwrap()).collect();
        
        format!("{}", serde_json::to_string(&deposit).unwrap())

    });

    server.listen("127.0.0.1:8675");

}
