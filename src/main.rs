mod client;
mod net;
mod protocol;

use client::acceptor::accept_new_clients;
use client::table::{ClientTable, SharedClientTable};
use crossbeam_channel::{Receiver, Sender};
use net::tap::{TapDevice, read_from_tap, write_to_tap};
use protocol::auth::{Auth, SharedAuth};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;

pub type ByteSender = Sender<Vec<u8>>;
pub type ByteReceiver = Receiver<Vec<u8>>;
type TapHandle = Arc<Box<dyn TapDevice>>;

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let tap: TapHandle = Arc::new(setup().unwrap());

    let table: SharedClientTable = Arc::new(ClientTable::new());

    let auth: SharedAuth = Arc::new(Mutex::new(Auth::new("/etc/blackwire")?));

    let (tap_tx, tap_rx) = crossbeam_channel::unbounded::<Vec<u8>>();

    start_threads(table, tap_tx, tap_rx, tap, auth);

    loop {}
}

fn setup() -> io::Result<Box<dyn TapDevice>> {
    println!("Setting up devices");

    let tap = net::tap::create_tap("bw0")?;

    println!("Created device `bw0`");

    #[cfg(target_os = "linux")]
    {
        net::bridge::linux::create("br0")?;
        net::bridge::linux::add_interface("br0", tap.name())?;
        net::bridge::linux::up("br0")?;
        net::bridge::linux::up(tap.name())?;
    }

    Ok(tap)
}

fn start_threads(
    table: SharedClientTable,
    tap_tx: ByteSender,
    tap_rx: ByteReceiver,
    tap: TapHandle,
    auth: SharedAuth,
) {
    let table_for_accepter = Arc::clone(&table);
    thread::spawn(move || {
        accept_new_clients(table_for_accepter, tap_tx, auth);
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
