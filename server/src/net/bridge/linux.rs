use std::io;
use std::process::Command;

pub fn add_qdisc(name: &str) -> io::Result<()> {
    Command::new("tc")
        .args(["qdisc", "add", "dev", name, "clsact"])
        .status()?;
    Ok(())
}

pub fn mirror_traffic(nic_a: &str, nic_b: &str) -> io::Result<()> {
    Command::new("tc")
        .args([
            "filter", "add", "dev", nic_a, "ingress", "matchall", "action", "mirred", "egress",
            "mirror", "dev", nic_b,
        ])
        .status()?;
    Ok(())
}
