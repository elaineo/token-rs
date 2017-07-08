extern crate serde;
extern crate serde_json;

#[macro_use] 
extern crate nickel;
#[macro_use]
extern crate serde_derive;

extern crate bincode;
extern crate leveldb;

use bincode::{serialize, deserialize, Infinite};

use std::path::Path;
use std::sync::Arc;

use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::iterator::Iterable;
use leveldb::options::{Options,WriteOptions,ReadOptions};

use nickel::status::StatusCode;
use nickel::{Nickel, HttpRouter};
mod raw_body;
use self::raw_body::*;

const DEFAULT_DIR: &'static str = "./tokendb";

#[derive(Serialize, Deserialize)]
pub struct SSDeposit {
    status: String,
    address: String,
    params: Vec<String>,
    id: usize,
}

fn main() {
    let mut server = Nickel::new();

    let path = Path::new(DEFAULT_DIR);
    let db = Arc::new(TokenDB::new(path));
    let read_db = db.clone();

    server.post("/", middleware! { |req, res| 
        let raw = req.raw_body();
        let deposit = serde_json::from_str::<SSDeposit>(&raw).unwrap();
        db.write_deposit(&deposit);
        
        format!("Deposit Received {} {}", deposit.address, deposit.status)
    });

    server.get("/:id", middleware! { |req|
        let id = req.param("id").unwrap();
        let key: i32 = id.parse()
                    .expect("Should be i32");

        let data = read_db.read_deposit(key);
        //deserialize(&data.unwrap()).unwrap()
    });

    /*
    server.utilize(router! {
        get "**" => |_req, _res| {
            let entry = SSDeposit {
                status: "new".to_string(),
                address: "elaine".to_string(),
                params: [].to_vec(),
                id: 1,
            };
            tokenDB.write_deposit(&entry);
            "Okay"
        }
    });*/

    server.listen("127.0.0.1:8675");

}

pub struct TokenDB {
  db: Database<i32>
}

impl TokenDB {
  pub fn new(path: &Path) -> TokenDB {
      let mut options = Options::new();
      options.create_if_missing = true;
      let db = match Database::open(path, options) {
        Ok(db) => { db },
        Err(e) => { panic!("failed to open database: {:?}", e) }
      };
      TokenDB {
        db: db,
      }
  }
  
  pub fn write_deposit(&self, deposit: &SSDeposit) -> () {
      let write_opts = WriteOptions::new();
      // turn into buffer
      let bytes: Vec<u8> = serialize(deposit, Infinite).unwrap();
      match self.db.put(write_opts, deposit.id as i32, &bytes) {
          Ok(_) => { () },
          Err(e) => { panic!("failed to write to database: {:?}", e) }
      };    
  }

  pub fn read_deposit(&self, key: i32) -> Option<Vec<u8>> {
      let read_opts = ReadOptions::new();
      let res = self.db.get(read_opts, key);
      let data = match res {
        Ok(data) => { data },
        Err(e) => { panic!("failed reading data: {:?}", e) }
      };
      data
      // deserialize(&data.unwrap()).unwrap(),
  } 

}
