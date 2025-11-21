/* This is the heart of the protocol:
 * Each message is preprending with an OPCODE -> Message format: [ OP ] [ DATA ]
 *
 * Valid OPCODES:
 * 0: Control
 * 1: Ethernet
 * 2: IPv4
 * 3: Error
 * 4: Disconnect
 *
 * Control packets have a further ControlType byte.
 * [ OP=0 ] [ TYPE ] [ DATA ]
 */

use std::convert::TryFrom;
use std::io;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Control = 0,
    Ethernet = 1,
    IP = 2,
}

impl TryFrom<u8> for OpCode {
    type Error = std::io::Error;

    fn try_from(value: u8) -> Result<Self, std::io::Error> {
        match value {
            0 => Ok(OpCode::Control),
            1 => Ok(OpCode::Ethernet),
            2 => Ok(OpCode::IP),
            _ => Err(io::Error::new(io::ErrorKind::Other, "Invalid OpCode")),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlType {
    Handshake = 0,
    AssignMac = 1,
    Pong = 2,
}

impl TryFrom<u8> for ControlType {
    type Error = std::io::Error;

    fn try_from(value: u8) -> Result<Self, std::io::Error> {
        match value {
            0 => Ok(ControlType::Handshake),
            1 => Ok(ControlType::AssignMac),
            2 => Ok(ControlType::Pong),
            _ => Err(io::Error::new(io::ErrorKind::Other, "Invalid OpCode")),
        }
    }
}

pub fn frame_ethernet(data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(OpCode::Ethernet as u8);
    msg.extend_from_slice(data);
    msg
}

pub fn frame_ip(data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(OpCode::IP as u8);
    msg.extend_from_slice(data);
    msg
}

pub fn classify_frame(data: &[u8]) -> io::Result<OpCode> {
    if data.len() < 1 {
        return Err(io::Error::new(io::ErrorKind::Other, "No data"));
    }
    OpCode::try_from(data[0])
}

pub fn parse_control_frame(data: &[u8]) -> io::Result<(ControlType, &[u8])> {
    if data.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Control frame too short",
        ));
    }

    let ctrl_type = ControlType::try_from(data[1])?;
    let payload = &data[2..];

    Ok((ctrl_type, payload))
}

pub fn frame_control(ctrl: ControlType, data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(OpCode::Control as u8);
    msg.push(ctrl as u8);
    msg.extend_from_slice(data);
    msg
}
