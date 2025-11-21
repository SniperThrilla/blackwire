use snow::{Builder, Keypair};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const NOISE_PARAMS: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
const PRIV_FILE: &str = "private.key";
const PUB_FILE: &str = "public.key";
const ALLOWED_DIR: &str = "allowed/";

pub type SharedAuth = Arc<Mutex<Auth>>;

pub struct Auth {
    pub keypair: Keypair,
    pub allowed: HashMap<String, Vec<u8>>,
    base: PathBuf,
    last_loaded: SystemTime,
}

impl Auth {
    pub fn new(base: impl AsRef<Path>) -> io::Result<Self> {
        let base = base.as_ref();

        check_keys_setup(base)?;
        Self::load(base)
    }

    pub fn load(base: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base.into();

        let private = read_hex(&base.join(PRIV_FILE))?;
        let public = read_hex(&base.join(PUB_FILE))?;

        let kp = Keypair { private, public };

        let allowed_path = base.join(ALLOWED_DIR);

        // Read in all the clients.
        let (map, mtime) = load_allowed_clients_with_mtime(&allowed_path)?;

        Ok(Self {
            keypair: kp,
            allowed: map,
            base,
            last_loaded: mtime,
        })
    }

    pub fn reload_if_modified(&mut self) -> io::Result<()> {
        let allowed_path = self.base.join(ALLOWED_DIR);

        if !allowed_path.exists() {
            return Ok(());
        }

        let m = fs::metadata(&allowed_path)?.modified()?;

        if m > self.last_loaded {
            let (map, mtime) = load_allowed_clients_with_mtime(&allowed_path)?;
            self.allowed = map;
            self.last_loaded = mtime;
            println!("Reloaded allowed client keys");
        }

        Ok(())
    }

    pub fn is_allowed(&self, key: &[u8]) -> bool {
        self.allowed.values().any(|k| k.as_slice() == key)
    }

    pub fn get_pub(&self, key: String) -> Option<&[u8]> {
        self.allowed.get(&key).map(|v| v.as_slice())
    }
}

fn load_allowed_clients_with_mtime(
    dir: &Path,
) -> io::Result<(HashMap<String, Vec<u8>>, SystemTime)> {
    let mut map = HashMap::new();

    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = entry.file_name().into_string().unwrap_or_default();

                if let Ok(data) = read_hex(&path) {
                    map.insert(filename, data);
                }
            }
        }
    }

    let mtime = fs::metadata(dir)?.modified()?;
    return Ok((map, mtime));
}

fn read_hex(path: &Path) -> io::Result<Vec<u8>> {
    let s = fs::read_to_string(path)?;
    let res = hex::decode(s.trim()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(res)
}

fn generate_static_keypair() -> Keypair {
    Builder::new(NOISE_PARAMS.parse().unwrap())
        .generate_keypair()
        .unwrap()
}

pub fn setup_keys_server(base: &Path) -> io::Result<()> {
    fs::create_dir_all(base.join(ALLOWED_DIR))?;

    // Create priv and public key files for the server.
    let kp = generate_static_keypair();

    let pub_hex = hex::encode(&kp.public);
    let priv_hex = hex::encode(&kp.private);

    fs::write(base.join(PRIV_FILE), priv_hex)?;
    fs::write(base.join(PUB_FILE), pub_hex)?;

    Ok(())
}

pub fn check_keys_setup(base: impl AsRef<Path>) -> io::Result<()> {
    let base = base.as_ref();

    // Check that all files exist
    let paths = [
        base.join(PRIV_FILE),
        base.join(PUB_FILE),
        base.join(ALLOWED_DIR),
    ];

    if paths.iter().any(|p| !p.exists()) {
        // This isn't fully set up
        setup_keys_server(base)?;
    }

    Ok(())
}
