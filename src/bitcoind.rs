use bitvmx_bitcoin_rpc::rpc_config::RpcConfig;
use bollard::errors::Error;
use bollard::models::{ContainerCreateBody, ContainerCreateResponse, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptions, CreateImageOptions, RemoveContainerOptions,
};
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::default::Default;
use tokio::runtime::Runtime;
use tracing::{self, debug, error, info};

use crate::config::BitcoindConfig;
use crate::error::BitcoindError;

pub struct Bitcoind {
    docker: Docker,
    container_name: String,
    image: String,
    hash: Option<String>,
    runtime: Runtime,
    rpc_config: RpcConfig,
    flags: BitcoindFlags,
}

#[derive(Debug, Clone)]
pub struct BitcoindFlags {
    pub min_relay_tx_fee: f64,
    pub block_min_tx_fee: f64,
    pub debug: u8,
    pub fallback_fee: f64,
    /// Maximum mempool size in MB. Default is None (uses bitcoind default of 300 MB).
    /// Set to a small value (e.g., 5) to limit mempool size for testing.
    pub maxmempool: Option<u32>,
}

impl Default for BitcoindFlags {
    fn default() -> Self {
        BitcoindFlags {
            min_relay_tx_fee: 0.00001,
            block_min_tx_fee: 0.00001,
            debug: 1,
            fallback_fee: 0.0002,
            maxmempool: None,
        }
    }
}

impl Bitcoind {
    /// Creates a new `Bitcoind` instance.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the Docker container.
    /// * `image` - The Docker image to use.
    /// * `hash` - Optional hash to verify the Docker image.
    /// * `rpc_config` - The RPC configuration for the Bitcoin node.
    /// * `flags` - Optional custom flags for the Bitcoin node.
    pub fn new(bitcoind_config: BitcoindConfig, rpc_config: RpcConfig, flags: Option<BitcoindFlags>) -> Self {
        let hash = match bitcoind_config.hash {
            Some(hash) => {
                let image_name = bitcoind_config.image.split(':').next().unwrap_or("");
                Some(format!("{}@{}", image_name, hash))
            }
            None => None,
        };

        let flags = flags.unwrap_or_else(BitcoindFlags::default);

        Self {
            docker: Docker::connect_with_local_defaults().unwrap(),
            container_name: bitcoind_config.container_name,
            image: bitcoind_config.image,
            hash,
            runtime: Runtime::new().unwrap(),
            rpc_config,
            flags,
        }
    }

    /// Starts the `bitcoind` Docker container.
    ///
    /// This method checks if the Docker daemon is active and then attempts to start
    /// the `bitcoind` container. If the container image is not found, it will pull
    /// the image and retry starting the container.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the container starts successfully.
    /// * `Err(Error)` if there is an error starting the container.
    pub fn start(&self) -> Result<(), BitcoindError> {
        info!("Checking if Docker daemon is active");
        let ping_result = self.runtime.block_on(async { self.docker.ping().await });

        if ping_result.is_err() {
            return Err(BitcoindError::DockerError(
                Error::DockerResponseServerError { status_code: 500
                    , message: "Docker deamon is not running. Make sure to start it before running this test".to_string() 
                }
            ));
        }

        info!("Starting bitcoind container");
        self.runtime.block_on(async {
            self.internal_stop().await?;

            let err = self.create_and_start_container().await;
            if let Err(err) = err {
                //FIX: For some reason checking the list of images is not working, so I handle the error here and retry.
                if err.to_string().contains("No such image")
                    || err.to_string().contains("Image hash mismatch")
                {
                    self.pull_image_if_not_present().await?;
                    self.create_and_start_container().await?;
                } else {
                    return Err(err);
                }
            }

            Ok(())
        })
    }

    /// Stops the `bitcoind` Docker container.
    ///
    /// This method stops the running `bitcoind` container by calling the internal
    /// stop method.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the container stops successfully.
    /// * `Err(Error)` if there is an error stopping the container.
    pub fn stop(&self) -> Result<(), BitcoindError> {
        info!("Stopping bitcoind container");
        self.runtime.block_on(async {
            self.internal_stop().await?;
            Ok(())
        })
    }

    async fn internal_stop(&self) -> Result<(), BitcoindError> {
        if self.is_running().await? {
            info!("Container was running. Stopping bitcoind container");
            self.docker
                .remove_container(
                    &self.container_name,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await?;
            for _ in 0..10 {
                if !self.is_running().await? {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                info!("Waiting for bitcoind container to stop");
            }
        }
        Ok(())
    }

    async fn is_running(&self) -> Result<bool, Error> {
        let containers = self
            .docker
            .list_containers(None::<bollard::query_parameters::ListContainersOptions>)
            .await?;
        for container in containers {
            if let Some(names) = container.names {
                if names.contains(&format!("/{}", self.container_name)) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn pull_image_if_not_present(&self) -> Result<(), BitcoindError> {
        info!("Image not found locally. Pulling image: {}", self.image);
        let options = Some(CreateImageOptions {
            from_image: Some(self.image.clone()),
            ..Default::default()
        });

        let mut stream = self.docker.create_image(options, None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(progress) => {
                    info!("Progress: {:?}", progress.progress);
                }
                Err(error) => {
                    return Err(BitcoindError::DockerError(error));
                }
            }
        }

        if let Some(hash) = &self.hash {
            debug!("Checking if image has hash: {}", hash);
            let image = self.docker.inspect_image(&self.image).await?;
            if let Some(digests) = image.repo_digests {
                if digests.contains(hash) {
                    info!("Image already has the required hash: {}", hash);
                } else {
                    error!("Image does not have the required hash: {}", hash);
                    return Err(BitcoindError::ImageHashMismatch {
                        expected: hash.clone(),
                        found: digests.join(", "),
                    });
                }
            }
        }

        Ok(())
    }

    async fn create_and_start_container(&self) -> Result<(), BitcoindError> {
        info!("Creating and starting bitcoind container");

        let min_relay_tx_fee = format!("-minrelaytxfee={}", self.flags.min_relay_tx_fee);
        let block_min_tx_fee = format!("-blockmintxfee={}", self.flags.block_min_tx_fee);
        let debug = format!("-debug={}", self.flags.debug);
        let fallback_fee = format!("-fallbackfee={}", self.flags.fallback_fee);

<<<<<<< Updated upstream
        if let Some(hash) = &self.hash {
            debug!("Checking if image has hash: {}", hash);
            let image = self.docker.inspect_image(&self.image).await?;
            if let Some(digests) = image.repo_digests {
                if digests.contains(hash) {
                    info!("Image already has the required hash: {}", hash);
                } else {
                    error!("Image does not have the required hash: {}", hash);
                    return Err(BitcoindError::ImageHashMismatch {
                        expected: hash.clone(),
                        found: digests.join(", "),
                    });
                }
            }
        }

        let config = ContainerCreateBody {
=======
        let mut cmd_args = vec![
            "-regtest=1".to_string(),
            "-printtoconsole".to_string(),
            "-rpcallowip=0.0.0.0/0".to_string(),
            "-rpcbind=0.0.0.0".to_string(),
            format!("-rpcuser={}", self.rpc_config.username.expose_secret()).to_string(),
            format!("-rpcpassword={}", self.rpc_config.password.expose_secret()).to_string(),
            "-server=1".to_string(),
            "-txindex=1".to_string(),
            debug,
            min_relay_tx_fee,
            block_min_tx_fee,
            fallback_fee,
        ];

        // Add maxmempool flag if specified
        if let Some(maxmempool_mb) = self.flags.maxmempool {
            cmd_args.push(format!("-maxmempool={}", maxmempool_mb));
        }

        let config = Config {
>>>>>>> Stashed changes
            image: Some(self.image.clone()),
            env: Some(vec!["BITCOIN_DATA=/data".to_string()]),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                port_bindings: Some(
                    [(
                        //TODO: Parse port from url
                        "18443/tcp".to_string(),
                        Some(vec![bollard::service::PortBinding {
                            host_ip: Some("0.0.0.0".to_string()),
                            host_port: Some("18443".to_string()),
                        }]),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                ..Default::default()
            }),
            cmd: Some(cmd_args),
            ..Default::default()
        };
        let ContainerCreateResponse { id, .. } = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: Some(self.container_name.clone()),
                    ..Default::default()
                }),
                config,
            )
            .await?;
        self.docker
            .start_container(
                &id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin::Network;
    use redact::Secret;

    #[test]
    fn test_start_stop_bitcoind() -> Result<(), BitcoindError> {
        let rpc_config = RpcConfig {
            username: Secret::new("foo".to_string()),
            password: Secret::new("rpcpassword".to_string()),
            url: Secret::new("http://localhost:18443".to_string()),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let config = BitcoindConfig::new(
            "bitcoin-regtest".to_string(),
            "bitcoin/bitcoin:29.1".to_string(),
            None,
        );

        let bitcoind = Bitcoind::new(config, rpc_config, None);

        bitcoind.start()?;
        bitcoind.stop()?;

        Ok(())
    }

    #[test]
    fn test_start_stop_bitcoind_with_flags() -> Result<(), BitcoindError> {
        let rpc_config = RpcConfig {
            username: Secret::new("foo".to_string()),
            password: Secret::new("rpcpassword".to_string()),
            url: Secret::new("http://localhost:18443".to_string()),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let flags = BitcoindFlags {
            min_relay_tx_fee: 0.00001,
            block_min_tx_fee: 0.00001,
            debug: 1,
            fallback_fee: 0.0002,
            maxmempool: None,
        };

        let config = BitcoindConfig::new(
            "bitcoin-regtest".to_string(),
            "bitcoin/bitcoin:29.1".to_string(),
            None,
        );

        let bitcoind = Bitcoind::new(config, rpc_config, Some(flags));

        bitcoind.start()?;
        bitcoind.stop()?;

        Ok(())
    }

    #[test]
    fn test_start_stop_bitcoind_with_correct_hash() -> Result<(), BitcoindError> {
        let rpc_config = RpcConfig {
            username: Secret::new("foo".to_string()),
            password: Secret::new("rpcpassword".to_string()),
            url: Secret::new("http://localhost:18443".to_string()),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let config = BitcoindConfig::new(
            "bitcoin-regtest".to_string(),
            "bitcoin/bitcoin:29.1".to_string(),
            Some(
                "sha256:de62c536feb629bed65395f63afd02e3a7a777a3ec82fbed773d50336a739319"
                    .to_string(),
            ),
        );

        let bitcoind = Bitcoind::new(config, rpc_config, None);

        bitcoind.start()?;
        bitcoind.stop()?;

        Ok(())
    }

    #[test]
    fn test_start_bitcoind_with_incorrect_hash() -> Result<(), BitcoindError> {
        let rpc_config = RpcConfig {
            username: Secret::new("foo".to_string()),
            password: Secret::new("rpcpassword".to_string()),
            url: Secret::new("http://localhost:18443".to_string()),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let config = BitcoindConfig::new(
            "bitcoin-regtest".to_string(),
            "bitcoin/bitcoin:29.1".to_string(),
            Some(
                "sha256:79dd32455cf8c268c63e5d0114cc9882a8857e942b1d17a6b8ec40a6d44e3981"
                    .to_string(),
            ),
        );

        let bitcoind = Bitcoind::new(config, rpc_config, None);

        assert!(bitcoind.start().is_err());

        Ok(())
    }
}
