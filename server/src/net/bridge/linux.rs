use std::io;
use std::process::Command;

pub fn create(name: &str) -> io::Result<()> {
    Command::new("ip")
        .args(["link", "add", name, "type", "bridge"])
        .status()?;
    Ok(())
}

pub fn add_interface(bridge: &str, iface: &str) -> io::Result<()> {
    Command::new("ip")
        .args(["link", "set", iface, "master", bridge])
        .status()?;
    Ok(())
}

pub fn up(name: &str) -> io::Result<()> {
    Command::new("ip")
        .args(["link", "set", name, "up"])
        .status()?;
    Ok(())
}
