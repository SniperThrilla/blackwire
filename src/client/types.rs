use crate::ByteSender;
use crate::net::mac::Mac;
use std::net::SocketAddr;

pub struct ClientInfo {
    pub mac: Mac,
    pub sender: ByteSender,
    pub addr: SocketAddr,
}
