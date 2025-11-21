use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use snow::TransportState;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub const NOISE_PARAMS: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

#[macro_export]
macro_rules! ok_or_continue {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                eprintln!("err: {}", e);
                continue;
            }
        }
    };
}

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

pub fn encrypt(transport: &mut TransportState, plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0u8; plaintext.len() + 1024];
    let n = transport
        .write_message(plaintext, &mut out)
        .map_err(io::Error::other)?;

    Ok(out[..n].to_vec())
}

pub fn decrypt(transport: &mut TransportState, ciphertext: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0u8; ciphertext.len() + 1024];
    let n = transport
        .read_message(ciphertext, &mut out)
        .map_err(io::Error::other)?;

    Ok(out[..n].to_vec())
}

pub fn safe_encrypt(
    transport: &Arc<Mutex<TransportState>>,
    plaintext: &[u8],
) -> io::Result<Vec<u8>> {
    let mut guard = transport.lock().unwrap();
    encrypt(&mut guard, plaintext)
}

pub fn safe_decrypt(
    transport: &Arc<Mutex<TransportState>>,
    ciphertext: &[u8],
) -> io::Result<Vec<u8>> {
    let mut guard = transport.lock().unwrap();
    decrypt(&mut guard, ciphertext)
}

pub fn recv_ciphertext(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let len = stream.read_u16::<BigEndian>()? as usize;

    let mut data = vec![0u8; len];
    stream.read_exact(&mut data)?;
    Ok(data)
}

pub fn send_ciphertext(stream: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    // NOTE: BlackWire uses framing with a 2 byte size leading data.
    stream.write_u16::<BigEndian>(msg.len() as u16)?;
    stream.write_all(msg)?;
    Ok(())
}
