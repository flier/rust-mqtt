use std::net::ToSocketAddrs;

pub async fn connect<A: ToSocketAddrs>(_addr: A) {}
