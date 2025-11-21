mod tap_impl;

#[cfg(target_os = "linux")]
mod linux;

use std::io;
use tap_impl::TapImpl;

pub struct Tap {
    inner: Box<dyn TapImpl>,
}

impl Tap {
    pub fn new(name: &str) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let backend = crate::linux::LinuxTap::new(name)?;
            return Ok(Self {
                inner: Box::new(backend),
            });
        }

        #[allow(unreachable_code)]
        Err(io::Error::new(io::ErrorKind::Other, "Unsupported OS"))
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    pub fn up(&self) -> io::Result<()> {
        self.inner.up()
    }

    pub fn set_mtu(&self, mtu: i32) -> io::Result<()> {
        self.inner.set_mtu(mtu)
    }

    pub fn set_mac(&self, mac: [u8; 6]) -> io::Result<()> {
        self.inner.set_mac(mac)
    }

    pub fn get_mac(&self) -> io::Result<[u8; 6]> {
        self.inner.get_mac()
    }

    pub fn ifname(&self) -> &str {
        self.inner.ifname()
    }
}
