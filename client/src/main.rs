use protocol::auth::Auth;
use protocol::framing::{ControlType, OpCode, classify_frame, parse_control_frame};
use protocol::noise::client::client_handshake;
use protocol::noise::util::recv_decrypted;
use snow::TransportState;
use std::io;
use std::net::TcpStream;

pub fn main() -> io::Result<()> {
    // Make a connection to the server.
    let addr = "127.0.0.1";
    let port = 9000;
    let mut stream = TcpStream::connect(format!("{}:{}", addr, port)).expect("Failed to connect");

    let auth: Auth = Auth::new("/etc/blackwire-client")?;

    let server_static = auth
        .get_pub("server".to_string())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No server public key found."))?;

    // Perform noise handshake
    let transport = client_handshake(&mut stream, &auth.keypair, server_static)?;

    println!("Noise handshake complete");

    // Perform protocol handshake
    blackwire_handshake(&mut stream, transport)?;

    // Set up TAP device

    // Set up threads

    loop {}
}

fn blackwire_handshake(stream: &mut TcpStream, mut transport: TransportState) -> io::Result<()> {
    let msg = recv_decrypted(stream, &mut transport)?;

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
    Ok(())
}
