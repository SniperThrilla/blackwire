use crate::client::table::SharedClientTable;
use crate::client::types::ClientInfo;
use crate::{ByteReceiver, ByteSender};
use protocol::auth::SharedAuth;
use protocol::framing::{ControlType, frame_control, frame_ethernet};
use protocol::noise::server::server_handshake;
use protocol::noise::util::{recv_decrypted, send_encrypted};
use snow::{Keypair, TransportState};
use std::io;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn client_thread(
    mut sock: TcpStream,
    table: SharedClientTable,
    tap_tx: ByteSender,
    auth: SharedAuth,
) -> io::Result<()> {
    let (tx_to_client, rx_from_tap) = crossbeam_channel::unbounded::<Vec<u8>>();

    // Get MAC for client and store this client.
    let addr = sock.peer_addr()?;
    let ci: Arc<ClientInfo> = table.add_new_client(addr, tx_to_client);

    println!("Assigned MAC {:02x?}", ci.mac);

    // Perform Noise handshake.
    let server_keypair = {
        let a = auth.lock().unwrap();
        Keypair {
            public: a.keypair.public.clone(),
            private: a.keypair.private.clone(),
        }
    };

    let (transport, client_static) = server_handshake(&mut sock, &server_keypair)?;
    let wrapped_transport = Arc::new(Mutex::new(transport));

    // Check the client static key is allowed.
    {
        auth.lock().unwrap().reload_if_modified()?;
    }
    {
        let locked_auth = auth.lock().unwrap();
        if !locked_auth.is_allowed(&client_static) {
            sock.shutdown(Shutdown::Both).ok();
            table.remove(ci.mac);
            drop(sock);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unauthorized client",
            ));
        }
    }

    // Perform BlackWire handshake.
    client_negotiation(&ci, &wrapped_transport, &mut sock)?;

    // Client is now ready to start transmitting data!
    // The event loop just deals with encrypting and forwarding to the client.
    let sock_writer = sock.try_clone().unwrap();
    let transport_writer = Arc::clone(&wrapped_transport);
    thread::spawn(move || client_write(sock_writer, rx_from_tap, transport_writer));

    client_read(sock, tap_tx, wrapped_transport);

    table.remove(ci.mac);

    Ok(())
}

fn client_negotiation(
    ci: &Arc<ClientInfo>,
    transport: &Arc<Mutex<TransportState>>,
    mut sock: &mut TcpStream,
) -> io::Result<()> {
    // Send MAC address to the client.
    let mac_frame = frame_control(ControlType::AssignMac, &ci.mac);
    let mut guard = transport.lock().unwrap();
    send_encrypted(&mut sock, &mut guard, &mac_frame)?;
    Ok(())
}

fn client_write(
    mut sock: TcpStream,
    data_stream: ByteReceiver,
    transport: Arc<Mutex<TransportState>>,
) {
    for frame in data_stream.iter() {
        let mut guard = transport.lock().unwrap();
        let framed = frame_ethernet(&frame);
        let _ = send_encrypted(&mut sock, &mut guard, &framed);
    }
}

fn client_read(
    mut sock: TcpStream,
    tap_channel: ByteSender,
    transport: Arc<Mutex<TransportState>>,
) {
    loop {
        let mut guard = transport.lock().unwrap();
        match recv_decrypted(&mut sock, &mut guard) {
            Ok(plain) => tap_channel.send(plain).unwrap(),
            Err(_) => {
                println!("Client disconnected or tampered");
                break;
            }
        }
    }
}
