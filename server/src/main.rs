mod client;
mod net;

use client::acceptor::accept_new_clients;
use client::table::{ClientTable, SharedClientTable};
use crossbeam_channel::{Receiver, Sender};
use net::tap::{read_from_tap, write_to_tap};
use protocol::auth::{Auth, SharedAuth};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use tap::Tap;

pub type ByteSender = Sender<Vec<u8>>;
pub type ByteReceiver = Receiver<Vec<u8>>;
type TapHandle = Arc<Tap>;

fn main() -> io::Result<()> {
    let ethernet_nic = "enp0s20f0u2";
    let port = 52123;

    let tap: TapHandle = Arc::new(setup(ethernet_nic).unwrap());

    let table: SharedClientTable = Arc::new(ClientTable::new());

    let auth: SharedAuth = Arc::new(Mutex::new(Auth::new("/etc/blackwire")?));

    let (tap_tx, tap_rx) = crossbeam_channel::unbounded::<Vec<u8>>();

    start_threads(table, tap_tx, tap_rx, tap, auth, port);

    loop {}
}

fn setup(nic: &str) -> io::Result<Tap> {
    println!("Setting up devices");

    let tap = Tap::new("bw0")?;
    tap.set_mtu(1400)?;
    tap.up()?;

    println!("Created device `bw0`");

    #[cfg(target_os = "linux")]
    {
        net::bridge::linux::add_qdisc("bw0")?;
        net::bridge::linux::add_qdisc(nic)?;
        net::bridge::linux::mirror_traffic("bw0", nic)?;
        net::bridge::linux::mirror_traffic(nic, "bw0")?;
    }

    Ok(tap)
}

fn start_threads(
    table: SharedClientTable,
    tap_tx: ByteSender,
    tap_rx: ByteReceiver,
    tap: TapHandle,
    auth: SharedAuth,
    port: u32,
) {
    let table_for_accepter = Arc::clone(&table);
    thread::spawn(move || {
        accept_new_clients(table_for_accepter, tap_tx, auth, port);
    });

    let table_for_writer = Arc::clone(&table);
    let tap_rx_for_writer = tap_rx.clone();
    let tap_for_writer = Arc::clone(&tap);
    thread::spawn(move || {
        write_to_tap(tap_for_writer, table_for_writer, tap_rx_for_writer);
    });

    let table_for_reader = Arc::clone(&table);
    let tap_for_reader = Arc::clone(&tap);
    thread::spawn(move || {
        read_from_tap(tap_for_reader, table_for_reader);
    });
}
