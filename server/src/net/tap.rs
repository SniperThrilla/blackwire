use super::mac::Mac;
use crate::client::table::SharedClientTable;
use crate::{ByteReceiver, TapHandle};
use protocol::ok_or_continue;

pub fn write_to_tap(tap: TapHandle, table: SharedClientTable, tap_rx: ByteReceiver) {
    for frame in tap_rx.iter() {
        {
            println!("Writing a frame to TAP!");
            /*match tap.write(&frame) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error writing to tap: {}", e);
                }
            };*/
        }
    }

    // Only reachable if tap_rx is finished.
}

pub fn read_from_tap(tap: TapHandle, table: SharedClientTable) {
    loop {
        let mut buf = [0u8; 2000];
        let n = match tap.read(&mut buf) {
            Ok(n) => n,
            Err(e) => {
                println!("Error reading from tap: {}", e);
                continue;
            }
        };

        if n < 14 {
            println!("Invalid ethernet frame");
            continue;
        }

        let frame = buf[..n].to_vec();

        let dst_mac: Mac = buf[0..6].try_into().unwrap();
        let src_mac: Mac = buf[6..12].try_into().unwrap();
        let tap_mac: Mac = ok_or_continue!(tap.get_mac());

        if src_mac == tap_mac {
            // This is OS generated data (we should ignore it!)
            println!("OS generated traffic ignored.");
            continue;
        }

        let broadcast: Mac = [0xff; 6];

        if dst_mac == broadcast {
            // Broadcast traffic
            println!("Broadcast traffic received from LAN.");
            for client in table.all_senders() {
                let _ = client.sender.send(frame.clone());
            }
            continue;
        } else if let Some(client_info) = table.get(dst_mac) {
            // Unicast traffic
            println!("Got traffic for client from LAN.");
            let _ = client_info.sender.send(buf.to_vec());
        } else {
            // Unknown unicast
        }
    }
}
