use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use snow::TransportState;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub const NOISE_PARAMS: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

pub fn read_msg(stream: &mut TcpStream, buf: &mut [u8]) -> io::Result<usize> {
    // NOTE: BlackWire uses framing with a 2 byte size leading data.
    let len = stream.read_u16::<BigEndian>()? as usize;
    stream.read_exact(&mut buf[..len])?;
    Ok(len)
}

pub fn write_msg(stream: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    // NOTE: BlackWire uses framing with a 2 byte size leading data.
    stream.write_u16::<BigEndian>(msg.len() as u16)?;
    stream.write_all(msg)?;
    Ok(())
}

pub fn send_encrypted(
    stream: &mut TcpStream,
    transport: &mut TransportState,
    plaintext: &[u8],
) -> io::Result<()> {
    let mut out = vec![0u8; plaintext.len() + 1024];

    let n = transport
        .write_message(plaintext, &mut out)
        .map_err(|e| io::Error::other(e))?;

    write_msg(stream, &mut out[..n])
}

pub fn recv_decrypted(
    stream: &mut TcpStream,
    transport: &mut TransportState,
) -> io::Result<Vec<u8>> {
    let mut in_buf = vec![0u8; 65535];
    let mut out = vec![0u8; 65535];

    let n = read_msg(stream, &mut in_buf)?;

    let m = transport
        .read_message(&in_buf[..n], &mut out)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(out[..m].to_vec())
}
