use bitcoin::Network;
use bitvmx_bitcoin_rpc::rpc_config::RpcConfig;
use redact::Secret;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct BitcoindConfig {
    pub container_name: String,
    pub image: String,
    pub hash: Option<String>,
    pub rpc_config: RpcConfig,
}

impl BitcoindConfig {
    pub fn new(
        container_name: String,
        image: String,
        hash: Option<String>,
        rpc_config: RpcConfig,
    ) -> Self {
        Self {
            container_name,
            image,
            hash,
            rpc_config,
        }
    }
}

impl Default for BitcoindConfig {
    fn default() -> Self {
        Self {
            container_name: "bitcoin-regtest".to_string(),
            image: "bitcoin/bitcoin:29.1".to_string(),
            hash: None,
            rpc_config: RpcConfig {
                username: Secret::new("foo".to_string()),
                password: Secret::new("rpcpassword".to_string()),
                url: Secret::new("http://localhost:18443".to_string()),
                wallet: "mywallet".to_string(),
                network: Network::Regtest,
            },
        }
    }
}