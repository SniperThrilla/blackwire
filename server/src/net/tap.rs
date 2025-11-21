use super::super::{ByteReceiver, TapHandle};
use super::mac::Mac;
use crate::client::table::SharedClientTable;
use std::io;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

pub trait TapDevice: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&self, buf: &[u8]) -> io::Result<usize>;
    fn name(&self) -> &str;
}

pub fn create_tap(name: &str) -> io::Result<Box<dyn TapDevice>> {
    #[cfg(target_os = "linux")]
    return linux::create_tap_linux(name);

    #[cfg(target_os = "macos")]
    return macos::create_tap_macos(name);

    #[cfg(target_os = "windows")]
    return windows::create_tap_windows(name);
}

pub fn write_to_tap(tap: TapHandle, table: SharedClientTable, tap_rx: ByteReceiver) {
    for frame in tap_rx.iter() {
        {
            match tap.write(&frame) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error writing to tap: {}", e);
                }
            };
        }
    }

    // Only reachable if tap_rx is finished.
}

pub fn read_from_tap(tap: TapHandle, table: SharedClientTable) {
    loop {
        let mut buf = [0u8; 2000];
        match tap.read(&mut buf) {
            Ok(_) => {}
            Err(e) => {
                println!("Error reading from tap: {}", e);
            }
        }

        //println!("Recieved packet: {:02x?}", &buf[..]);

        // Inspect the MAC to see which client it should go to.
        let mac: Mac = [0u8; 6]; // TODO: Actually read the MAC.
        if let Some(client_info) = table.get(mac) {
            println!("Got traffic for client.");
            match client_info.sender.send(buf.to_vec()) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error sending frame to client thread: {}", e);
                }
            };
        } else {
            //println!("Unknown recipient (likely broadcast traffic)");
        }
    }
}
