use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub keypair: Keypair,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Keypair {
    pub public: [u8; 32],
    pub private: [u8; 32],
}

pub async fn init() -> Config {
    if Path::new("pantomath.toml").exists() {
        let mut config = Vec::with_capacity(4096);
        File::open("pantomath.toml")
            .await
            .unwrap()
            .read_to_end(&mut config)
            .await
            .unwrap();
        return toml::from_slice(config.as_slice()).unwrap();
    }
    let cfg = Config {
        keypair: snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
            .generate_keypair()
            .unwrap()
            .into(),
    };
    File::create("pantomath.toml")
        .await
        .unwrap()
        .write_all(toml::to_vec(&cfg).unwrap().as_slice())
        .await
        .unwrap();
    cfg
}

impl Into<snow::Keypair> for Keypair {
    fn into(self) -> snow::Keypair {
        snow::Keypair {
            public: self.public.to_vec(),
            private: self.private.to_vec(),
        }
    }
}

impl From<snow::Keypair> for Keypair {
    fn from(kp: snow::Keypair) -> Self {
        let mut public = [0u8; 32];
        let mut private = [0u8; 32];
        public.copy_from_slice(kp.public.as_slice());
        private.copy_from_slice(kp.private.as_slice());
        Self { public, private }
    }
}
