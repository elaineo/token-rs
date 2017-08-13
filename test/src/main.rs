#[macro_use] 
extern crate nickel;

use nickel::{Nickel, HttpRouter};


fn main() {
    let mut server = Nickel::new();

    server.get("/txStat/:address", middleware! { |req|
        let address = req.param("address").unwrap();
        println!("address: {}", address);
        format!("{{\"status\" : \"complete\",
            \"address\": \"{}\",
            \"withdraw\": \"0x43633c233d88A5cDbAFC9c4F72E24FA1039e6449\",
            \"incomingCoin\": \"123\",
            \"incomingType\": \"bch\",
            \"outgoingCoin\": \"1.5\",
            \"outgoingType\": \"etc\",
            \"transaction\": \"xxxxx-xxxx\"
        }}", address)
    });
    server.listen("127.0.0.1:8008");

}
