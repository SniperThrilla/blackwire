use crate::ByteSender;
use crate::client::handler::client_thread;
use crate::client::table::SharedClientTable;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

pub fn accept_new_clients(table: SharedClientTable, tap_tx: ByteSender) {
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
