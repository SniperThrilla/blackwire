use crate::ByteSender;
use crate::client::handler::client_thread;
use crate::client::table::SharedClientTable;
use protocol::auth::SharedAuth;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

pub fn accept_new_clients(
    table: SharedClientTable,
    tap_tx: ByteSender,
    auth: SharedAuth,
    port: u32,
) {
    // Accept new clients, these are clients joining the LAN
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("Listening on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(sock) => {
                println!("New client {:?}", sock.peer_addr());

                let table_for_client = Arc::clone(&table);
                let tap_tx_for_client = tap_tx.clone();
                let auth_for_client = Arc::clone(&auth);
                thread::spawn(move || {
                    match client_thread(sock, table_for_client, tap_tx_for_client, auth_for_client)
                    {
                        Ok(_) => {}
                        Err(e) => {
                            println!("A client has errored: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    loop {}
}
