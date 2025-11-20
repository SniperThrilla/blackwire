use crate::client::table::SharedClientTable;
use crate::client::types::ClientInfo;
use crate::{ByteReceiver, ByteSender};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;

pub fn client_thread(sock: TcpStream, table: SharedClientTable, tap_tx: ByteSender) {
    let (tx_to_client, rx_from_tap) = crossbeam_channel::unbounded::<Vec<u8>>();

    // Get MAC for client and store this client.
    let addr = sock.peer_addr().unwrap();
    let ci: Arc<ClientInfo> = table.add_new_client(addr, tx_to_client);

    println!("Assigned MAC {:02x?}", ci.mac);

    // Perform Noise handshake.
    client_noise_handshake();

    // Perform BlackWire handshake.
    client_negotiation();

    // Client is now ready to start transmitting data!
    // The event loop just deals with encrypting and forwarding to the client.
    let sock_writer = sock.try_clone().unwrap();
    thread::spawn(move || client_write(sock_writer, rx_from_tap));

    client_read(sock, tap_tx);

    table.remove(ci.mac);
}

fn client_noise_handshake() {
    todo!();
}

fn client_negotiation() {
    todo!();
}

fn client_write(mut sock: TcpStream, data_stream: ByteReceiver) {
    for frame in data_stream.iter() {
        // TODO: Encryption
        sock.write_all(&frame).unwrap();
    }
}

fn client_read(mut sock: TcpStream, tap_channel: ByteSender) {
    let mut buf = [0u8; 2000];

    loop {
        let n = sock.read(&mut buf).unwrap();
        if n == 0 {
            println!("Client disconnected");
            break;
        }

        let frame = buf[..n].to_vec();
        // TODO:Decryption
        tap_channel.send(frame).unwrap();
    }
}
