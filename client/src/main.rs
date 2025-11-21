use protocol::auth::Auth;
use protocol::framing::{ControlType, OpCode, classify_frame, frame_ethernet, parse_control_frame};
use protocol::noise::client::client_handshake;
use protocol::noise::util::{
    decrypt, recv_ciphertext, safe_decrypt, safe_encrypt, send_ciphertext,
};
use protocol::ok_or_continue;
use snow::TransportState;
use std::io;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use tap::Tap;

pub fn main() -> io::Result<()> {
    let addr = "127.0.0.1";
    let port = 9000;

    let auth: Auth = Auth::new("/etc/blackwire-client")?;

    let server_static = auth
        .get_pub("server".to_string())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No server public key found."))?;

    // Make a connection to the server.
    let mut stream = TcpStream::connect(format!("{}:{}", addr, port)).expect("Failed to connect");

    // Perform noise handshake
    let mut transport = client_handshake(&mut stream, &auth.keypair, server_static)?;

    println!("Noise handshake complete");

    // Perform protocol handshake
    let mac = blackwire_handshake(&mut stream, &mut transport)?;

    // Set up TAP device
    let tap = Tap::new("bwc0")?;
    tap.set_mtu(1400)?;
    tap.set_mac(mac)?;
    tap.up()?;

    let shared_tap = Arc::new(tap);
    let shared_transport = Arc::new(Mutex::new(transport));

    // Set up threads
    let read_tap = Arc::clone(&shared_tap);
    let read_transport = Arc::clone(&shared_transport);
    let mut read_stream = stream.try_clone()?;
    thread::spawn(move || {
        read_from_tap(read_tap, &mut read_stream, read_transport);
    });

    let stream_tap = Arc::clone(&shared_tap);
    let stream_transport = Arc::clone(&shared_transport);
    let mut recv_stream = stream.try_clone()?;
    thread::spawn(move || {
        read_from_stream(stream_tap, &mut recv_stream, stream_transport);
    });

    loop {}
}

fn blackwire_handshake(
    stream: &mut TcpStream,
    mut transport: &mut TransportState,
) -> io::Result<[u8; 6]> {
    let ciphertext = recv_ciphertext(stream)?;
    let msg = decrypt(&mut transport, &ciphertext)?;

    let opcode = classify_frame(&msg)?;
    if opcode != OpCode::Control {
        return Err(io::Error::new(io::ErrorKind::Other, "Incorrect handshake"));
    }

    let (control_type, mac) = parse_control_frame(&msg)?;
    if control_type != ControlType::AssignMac {
        return Err(io::Error::new(io::ErrorKind::Other, "Incorrect handshake"));
    }

    // Verify that the payload is 6 bytes (it's a MAC address)
    if mac.len() != 6 {
        return Err(io::Error::new(io::ErrorKind::Other, "Incorrect handshake"));
    }

    println!("Received MAC address! {:02x?}", mac);
    let mac_arr: [u8; 6] = mac.try_into().map_err(io::Error::other)?;
    Ok(mac_arr)
}

fn read_from_tap(tap: Arc<Tap>, stream: &mut TcpStream, transport: Arc<Mutex<TransportState>>) {
    loop {
        // Read ethernet frame from TAP.
        let mut buf = [0u8; 2000];
        let size = ok_or_continue!(tap.read(&mut buf));
        println!("Got an ethernet frame from the TAP!");

        // Frame ethernet frame.
        let frame = frame_ethernet(&buf[..size]);

        // Encrypt and send ethernet frame.
        let ciphertext = ok_or_continue!(safe_encrypt(&transport, &frame));
        ok_or_continue!(send_ciphertext(stream, &ciphertext));
    }
}

fn read_from_stream(tap: Arc<Tap>, stream: &mut TcpStream, transport: Arc<Mutex<TransportState>>) {
    loop {
        // Read and decrypt message from TcpStream
        let ciphertext = ok_or_continue!(recv_ciphertext(stream));
        let data = ok_or_continue!(safe_decrypt(&transport, &ciphertext));
        // Classify message
        match ok_or_continue!(classify_frame(&data)) {
            OpCode::Control => {
                let (ctrl_type, payload) = ok_or_continue!(parse_control_frame(&data));
                match ctrl_type {
                    ControlType::Handshake => {}
                    ControlType::AssignMac => {}
                    ControlType::Pong => {}
                }
            }

            OpCode::Ethernet => {
                // Send this to TAP
                println!("Received ethernet from server, writing it out TAP!");
                let ethernet = &data[1..];
                ok_or_continue!(tap.write(&ethernet));
            }
            OpCode::IP => {}
        }
    }
}
