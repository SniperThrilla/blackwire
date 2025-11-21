#![cfg(target_os = "linux")]

use crate::TapImpl;
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

impl LinuxTap {
    pub fn new(name: &str) -> io::Result<Self> {
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
        let ret =
            unsafe { libc::ioctl(fd, libc::TUNSETIFF, &mut ifr as *mut _ as *mut libc::c_void) };

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

        Ok(Self {
            fd,
            fd_write,
            name: actual,
        })
    }
}

impl TapImpl for LinuxTap {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        nix::unistd::read(self.fd, buf).map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let bfd = unsafe { BorrowedFd::borrow_raw(self.fd_write) };
        nix::unistd::write(bfd, buf).map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn up(&self) -> io::Result<()> {
        unsafe {
            // Open a control socket.
            let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if sock < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut ifr = ifreq_for(&self.name);

            // Get current flags
            if libc::ioctl(sock, libc::SIOCGIFFLAGS, &mut ifr) < 0 {
                return Err(io::Error::last_os_error());
            }

            // Update flags
            let mut flags = ifr.ifr_ifru.ifru_flags as libc::c_int;

            flags |= libc::IFF_UP | libc::IFF_RUNNING;

            ifr.ifr_ifru.ifru_flags = flags as libc::c_short;

            // Set to updated flags
            if libc::ioctl(sock, libc::SIOCSIFFLAGS, &ifr) < 0 {
                return Err(io::Error::last_os_error());
            }

            libc::close(sock);
            Ok(())
        }
    }

    fn set_mtu(&self, mtu: i32) -> io::Result<()> {
        unsafe {
            let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if sock < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut ifr = ifreq_for(&self.name);

            ifr.ifr_ifru.ifru_ivalue = mtu;

            if libc::ioctl(sock, libc::SIOCSIFMTU, &ifr) < 0 {
                return Err(io::Error::last_os_error());
            }

            libc::close(sock);
            Ok(())
        }
    }

    fn set_mac(&self, mac: [u8; 6]) -> io::Result<()> {
        unsafe {
            let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if sock < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut ifr = ifreq_for(&self.name);

            let mut addr: libc::sockaddr = std::mem::zeroed();
            addr.sa_family = libc::ARPHRD_ETHER as libc::sa_family_t;

            for i in 0..6 {
                addr.sa_data[i] = mac[i] as libc::c_char;
            }

            ifr.ifr_ifru.ifru_addr = addr;

            if libc::ioctl(sock, libc::SIOCSIFHWADDR, &ifr) < 0 {
                return Err(io::Error::last_os_error());
            }

            libc::close(sock);
            Ok(())
        }
    }

    fn get_mac(&self) -> io::Result<[u8; 6]> {
        unsafe {
            let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if sock < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut ifr = ifreq_for(&self.name);

            for (i, b) in self.ifname().bytes().enumerate() {
                ifr.ifr_name[i] = b as libc::c_char;
            }

            if libc::ioctl(sock, libc::SIOCGIFHWADDR, &ifr) < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut mac = [0u8; 6];
            for i in 0..6 {
                mac[i] = ifr.ifr_ifru.ifru_addr.sa_data[i] as u8;
            }

            libc::close(sock);
            Ok(mac)
        }
    }

    fn ifname(&self) -> &str {
        &self.name
    }
}

fn ifreq_for(name: &str) -> Ifreq {
    let mut ifr: Ifreq = unsafe { std::mem::zeroed() };

    for (i, b) in name.bytes().enumerate() {
        ifr.ifr_name[i] = b as libc::c_char;
    }

    ifr
}
