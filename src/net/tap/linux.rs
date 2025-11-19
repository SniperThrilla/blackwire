#![cfg(target_os = "linux")]

use super::TapDevice;
use libc::{IFF_NO_PI, IFF_TAP};
use nix::fcntl::{OFlag, open};
use nix::ioctl_write_ptr;
use nix::sys::stat::Mode;
use nix::unistd::dup;
use std::ffi::{CStr, CString};
use std::io;
use std::mem;
use std::os::unix::io::{BorrowedFd, RawFd};

ioctl_write_ptr!(tun_set_iff, b'T', 202, Ifreq);

pub struct LinuxTap {
    fd: RawFd,
    fd_write: RawFd,
    name: String,
}

#[repr(C)]
pub union Ifru {
    pub ifru_flags: libc::c_short,
    pub ifru_ivalue: libc::c_int,
    pub ifru_addr: libc::sockaddr,
}

#[repr(C)]
pub struct Ifreq {
    pub ifr_name: [libc::c_char; libc::IFNAMSIZ],
    pub ifr_ifru: Ifru,
}

impl TapDevice for LinuxTap {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        nix::unistd::read(self.fd, buf).map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let bfd = unsafe { BorrowedFd::borrow_raw(self.fd_write) };
        nix::unistd::write(bfd, buf).map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn create_tap_linux(name: &str) -> io::Result<Box<dyn TapDevice>> {
    // Open FD to /dev/net/tun
    let fd = open("/dev/net/tun", OFlag::O_RDWR, Mode::empty())
        .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

    // Set up ifreq C struct.
    let mut ifr: Ifreq = unsafe { mem::zeroed() };
    let c_name = CString::new(name)?;
    let bytes = c_name.as_bytes_with_nul();
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            ifr.ifr_name.as_mut_ptr() as *mut u8,
            bytes.len(),
        );
    }
    ifr.ifr_ifru.ifru_flags = (IFF_TAP | IFF_NO_PI) as libc::c_short;

    // Call ioctl(TUNSETIFF)
    let ret = unsafe { libc::ioctl(fd, libc::TUNSETIFF, &mut ifr as *mut _ as *mut libc::c_void) };
    println!("ioctl ret = {}", ret);
    if ret < 0 {
        println!("errno = {:?}", io::Error::last_os_error());
        return Err(io::Error::last_os_error());
    }

    // Get the actual name back from kernel
    let actual = unsafe {
        CStr::from_ptr(ifr.ifr_name.as_ptr())
            .to_str()
            .unwrap()
            .to_string()
    };

    let fd_write = dup(fd).unwrap();

    let tap = LinuxTap {
        fd,
        fd_write,
        name: actual,
    };

    Ok(Box::new(tap) as Box<dyn TapDevice>)
}
