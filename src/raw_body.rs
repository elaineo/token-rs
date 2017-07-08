//! Provides a plugin for Nickel's Request which allows the raw contents
//! of a body to be made available via the req.raw_body() method.
//
// I found it on the internet: https://github.com/nickel-org/nickel.rs/issues/359

extern crate plugin;
extern crate typemap;

use std::io;
use std::io::Read;
use nickel::Request;
use self::typemap::Key;
use self::plugin::{Plugin, Pluggable};

struct RawBodyPlugin;

pub trait RawBody {
    fn raw_body(&mut self) -> &str;
}

impl Key for RawBodyPlugin { type Value = String; }

impl<'mw, 'conn, D> Plugin<Request<'mw, 'conn, D>> for RawBodyPlugin {
    type Error = io::Error;

    fn eval(req: &mut Request<D>) -> Result<String, io::Error> {
        let mut buffer = String::new();
        try!(req.origin.read_to_string(&mut buffer));
        Ok(buffer)
    }
}

impl<'mw, 'conn, D> RawBody for Request<'mw, 'conn, D> {
    fn raw_body(&mut self) -> &str {
        match self.get_ref::<RawBodyPlugin>().ok() {
            Some(x) => x,
            None => "",
        }
    }
}