use std::io;
use std::marker::PhantomData;

use futures::{Future, future};
use tokio_service::Service;

use core::Packet;

/// MQTT service
#[derive(Debug, Default)]
pub struct MQTT3<'a>(PhantomData<&'a u8>);

impl<'a> Service for MQTT3<'a> {
    type Request = Packet<'a>;
    type Response = Packet<'a>;

    type Error = io::Error;

    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        Box::new(future::ok(Packet::Disconnect))
    }
}
