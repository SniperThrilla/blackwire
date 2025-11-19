mod net;

use crossbeam_channel::{Receiver, Sender};
use net::tap::TapDevice;
use rand::RngCore;
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

type Mac = [u8; 6];
type ClientSender = Sender<Vec<u8>>;
type ClientTable = Arc<Mutex<HashMap<Mac, ClientSender>>>;
type TapHandle = Arc<Box<dyn TapDevice>>;

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let tap: TapHandle = Arc::new(setup().unwrap());

    let table: ClientTable = Arc::new(Mutex::new(HashMap::new()));
    let (tap_tx, tap_rx) = crossbeam_channel::unbounded::<Vec<u8>>();

    let table_for_accepter = Arc::clone(&table);
    thread::spawn(move || {
        accept_new_clients(table_for_accepter, tap_tx);
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

    loop {}
}

fn accept_new_clients(table: ClientTable, tap_tx: ClientSender) {
    // Accept new clients, these are clients joining the LAN
    let port = 9000;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("Listening on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(sock) => {
                println!("New client {:?}", sock.peer_addr());

                let table_for_client = Arc::clone(&table);
                let tap_tx_for_client = tap_tx.clone();
                thread::spawn(move || {
                    client_thread(sock, table_for_client, tap_tx_for_client);
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    loop {}
}

fn generate_mac(table: &ClientTable) -> Mac {
    loop {
        let mac = random_mac();

        let map = table.lock().unwrap();
        if !map.contains_key(&mac) {
            return mac;
        }
    }
}

fn random_mac() -> Mac {
    let mut rng = rand::thread_rng();
    let mut mac = [0u8; 6];

    rng.fill_bytes(&mut mac);

    mac[0] &= 0b11111110; // Clear multicast bit
    mac[0] |= 0b00000010; // Set locally administered bit

    mac
}

fn write_to_tap(tap: TapHandle, table: ClientTable, tap_rx: Receiver<Vec<u8>>) {
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

fn read_from_tap(tap: TapHandle, table: ClientTable) {
    loop {
        let mut buf = [0u8; 2000];
        match tap.read(&mut buf) {
            Ok(_) => {}
            Err(e) => {
                println!("Error reading from tap: {}", e);
            }
        }

        println!("Recieved packet: {:02x?}", &buf[..]);

        // Inspect the MAC to see which client it should go to.
        let mac: Mac = [0u8; 6]; // TODO: Actually read the MAC.
        {
            let map = table.lock().unwrap();
            if let Some(client_tx) = map.get(&mac) {
                match client_tx.send(buf.to_vec()) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error sending frame to client thread: {}", e);
                    }
                };
            } else {
                println!("Packet received but dropped because no matching MAC.");
            }
        }
    }
}

fn client_thread(mut sock: TcpStream, table: ClientTable, tap_tx: ClientSender) {
    let (tx_to_client, rx_from_tap) = crossbeam_channel::unbounded::<Vec<u8>>();

    let mac = generate_mac(&table);
    println!("Assigned MAC {:02x?}", mac);

    // Perform Noise handshake.

    // Update the hashmap with client values
    {
        let mut map = table.lock().unwrap();
        map.insert(mac, tx_to_client.clone());
    }

    // Perform BlackWire handshake.

    // Client is now ready to start transmitting data!
    // The event loop just deals with encrypting and forwarding to the client.
    let mut sock_writer = sock.try_clone().unwrap();
    thread::spawn(move || {
        for frame in rx_from_tap.iter() {
            // TODO: Encryption
            sock_writer.write_all(&frame).unwrap();
        }

        // Writer thread ends if channel closes.
    });

    let mut buf = [0u8; 2000];

    loop {
        let n = sock.read(&mut buf).unwrap();
        if n == 0 {
            println!("Client disconnected");
            break;
        }

        let frame = buf[..n].to_vec();
        // TODO:Decryption
        tap_tx.send(frame).unwrap();
    }

    {
        let mut map = table.lock().unwrap();
        map.remove(&mac);
    }
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
