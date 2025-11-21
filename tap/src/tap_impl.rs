use std::io;

pub trait TapImpl: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&self, buf: &[u8]) -> io::Result<usize>;
    fn up(&self) -> io::Result<()>;
    fn set_mtu(&self, mtu: i32) -> io::Result<()>;
    fn set_mac(&self, mac: [u8; 6]) -> io::Result<()>;
    fn get_mac(&self) -> io::Result<[u8; 6]>;
    fn ifname(&self) -> &str;
}
