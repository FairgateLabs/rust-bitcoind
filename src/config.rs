use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct BitcoindConfig {
    pub container_name: String,
    pub image: String,
    pub hash: Option<String>,
}

impl BitcoindConfig {
    pub fn new(
        container_name: String,
        image: String,
        hash: Option<String>,
    ) -> Self {
        Self {
            container_name,
            image,
            hash,
        }
    }
}

impl Default for BitcoindConfig {
    fn default() -> Self {
        Self {
            container_name: "bitcoin-regtest".to_string(),
            image: "bitcoin/bitcoin:29.1".to_string(),
            hash: Some("sha256:de62c536feb629bed65395f63afd02e3a7a777a3ec82fbed773d50336a739319".to_string()),
        }
    }
}