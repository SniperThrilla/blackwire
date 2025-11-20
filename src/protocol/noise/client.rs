use crate::protocol::noise::util::{NOISE_PARAMS, read_msg, write_msg};
use snow::{Builder, Keypair, TransportState};
use std::io;
use std::net::TcpStream;

pub fn client_handshake(
    stream: &mut TcpStream,
    client_static: &Keypair,
    server_pub: &[u8],
) -> io::Result<TransportState> {
    let builder = Builder::new(NOISE_PARAMS.parse().unwrap())
        .local_private_key(&client_static.private)
        .map_err(|e| io::Error::other(e))?
        .remote_public_key(server_pub)
        .map_err(|e| io::Error::other(e))?;

    let mut noise = builder.build_initiator().unwrap();

    let mut in_buf = [0u8; 65535];
    let mut out_buf = [0u8; 65535];

    let client_msg_len = noise
        .write_message(&client_static.public, &mut out_buf)
        .map_err(|e| io::Error::other(e))?;
    write_msg(stream, &mut out_buf[..client_msg_len])?;

    let server_msg_len = read_msg(stream, &mut in_buf)?;
    noise
        .read_message(&in_buf[..server_msg_len], &mut out_buf)
        .map_err(|e| io::Error::other(e))?;

    let transport = noise.into_transport_mode().unwrap();

    Ok(transport)
}
