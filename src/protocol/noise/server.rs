use crate::protocol::noise::util::{NOISE_PARAMS, read_msg, write_msg};
use snow::{Builder, Keypair, TransportState};
use std::io;
use std::net::TcpStream;

pub fn server_handshake(
    stream: &mut TcpStream,
    server_static: &Keypair,
) -> io::Result<(TransportState, Vec<u8>)> {
    // Build Noise Responder
    let builder = Builder::new(NOISE_PARAMS.parse().unwrap())
        .local_private_key(&server_static.private)
        .map_err(|e| io::Error::other(e))?;

    let mut noise = builder.build_responder().unwrap();

    let mut in_buf = [0u8; 65535];
    let mut out_buf = [0u8; 65535];

    // Receive message from client.
    let client_message_len = read_msg(stream, &mut in_buf)?;
    let payload_len = noise
        .read_message(&in_buf[..client_message_len], &mut out_buf)
        .map_err(|e| io::Error::other(e))?;

    // Extract client static key (used for authentication)
    let client_static_pubkey = out_buf[..payload_len].to_vec();

    // Send server response
    let response_message_len = noise
        .write_message(&[], &mut out_buf)
        .map_err(|e| io::Error::other(e))?;
    write_msg(stream, &mut out_buf[..response_message_len])?;

    let transport = noise.into_transport_mode().unwrap();

    Ok((transport, client_static_pubkey))
}
