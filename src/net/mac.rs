use rand::RngCore;

pub type Mac = [u8; 6];

pub fn generate_mac<'a, I>(existing: &mut I) -> Mac
where
    I: Clone + Iterator<Item = &'a Mac>,
{
    loop {
        let mac = random_mac();

        if !existing.clone().any(|m| m == &mac) {
            return mac;
        }
    }
}

fn random_mac() -> Mac {
    let mut rng = rand::thread_rng();
    let mut mac = [0u8; 6];

    rng.fill_bytes(&mut mac);

    mac[0] &= 0b11111110; // Clear multicast bit
    mac[0] |= 0b00000010; // Set locally administered bit

    mac
}
