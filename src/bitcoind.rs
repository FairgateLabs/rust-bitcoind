use bitvmx_bitcoin_rpc::rpc_config::RpcConfig;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::errors::Error;
use bollard::image::CreateImageOptions;
use bollard::models::{ContainerCreateResponse, HostConfig};
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::default::Default;
use tokio::runtime::Runtime;
use tracing::{self, info};

pub struct Bitcoind {
    docker: Docker,
    container_name: String,
    image: String,
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
}

impl Default for BitcoindFlags {
    fn default() -> Self {
        BitcoindFlags {
            min_relay_tx_fee: 0.00001,
            block_min_tx_fee: 0.00001,
            debug: 1,
            fallback_fee: 0.0002,
        }
    }
}

impl Bitcoind {
    /// Creates a new `Bitcoind` instance with default flags.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the Docker container.
    /// * `image` - The Docker image to use.
    /// * `rpc_config` - The RPC configuration for the Bitcoin node.
    pub fn new(container_name: &str, image: &str, rpc_config: RpcConfig) -> Self {
        Self::new_with_flags(container_name, image, rpc_config, BitcoindFlags::default())
    }

    /// Creates a new `Bitcoind` instance with specified flags.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the Docker container.
    /// * `image` - The Docker image to use.
    /// * `rpc_config` - The RPC configuration for the Bitcoin node.
    /// * `flags` - Custom flags for the Bitcoin node.
    pub fn new_with_flags(
        container_name: &str,
        image: &str,
        rpc_config: RpcConfig,
        flags: BitcoindFlags,
    ) -> Self {
        Self {
            docker: Docker::connect_with_local_defaults().unwrap(),
            container_name: container_name.to_string(),
            image: image.to_string(),
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
    pub fn start(&self) -> Result<(), Error> {
        info!("Checking if Docker daemon is active");
        let ping_result = self.runtime.block_on(async { self.docker.ping().await });

        if ping_result.is_err() {
            return Err(Error::DockerResponseNotFoundError {
                message:
                    "Docker deamon is not running. Make sure to start it before running this test"
                        .to_string(),
            });
        }

        info!("Starting bitcoind container");
        self.runtime.block_on(async {
            self.internal_stop().await?;

            let err = self.create_and_start_container().await;
            if let Err(err) = err {
                //FIX: For some reason checking the list of images is not working, so I handle the error here and retry.
                if err.to_string().contains("No such image") {
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
    pub fn stop(&self) -> Result<(), Error> {
        info!("Stopping bitcoind container");
        self.runtime.block_on(async {
            self.internal_stop().await?;
            Ok(())
        })
    }

    async fn internal_stop(&self) -> Result<(), Error> {
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
            .list_containers(None::<bollard::container::ListContainersOptions<String>>)
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

    async fn pull_image_if_not_present(&self) -> Result<(), Error> {
        info!("Image not found locally. Pulling image: {}", self.image);
        let options = Some(CreateImageOptions {
            from_image: self.image.clone(),
            tag: "latest".to_string(),
            ..Default::default()
        });

        let mut stream = self.docker.create_image(options, None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(progress) => {
                    info!("Progress: {:?}", progress.progress);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }

        Ok(())
    }

    async fn create_and_start_container(&self) -> Result<(), Error> {
        info!("Creating and starting bitcoind container");

        let min_relay_tx_fee = format!("-minrelaytxfee={}", self.flags.min_relay_tx_fee);
        let block_min_tx_fee = format!("-blockmintxfee={}", self.flags.block_min_tx_fee);
        let debug = format!("-debug={}", self.flags.debug);
        let fallback_fee = format!("-fallbackfee={}", self.flags.fallback_fee);

        let config = Config {
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
            cmd: Some(vec![
                "-regtest=1".to_string(),
                "-printtoconsole".to_string(),
                "-rpcallowip=0.0.0.0/0".to_string(),
                "-rpcbind=0.0.0.0".to_string(),
                format!("-rpcuser={}", self.rpc_config.username).to_string(),
                format!("-rpcpassword={}", self.rpc_config.password).to_string(),
                "-server=1".to_string(),
                "-txindex=1".to_string(),
                debug,
                min_relay_tx_fee,
                block_min_tx_fee,
                fallback_fee,
            ]),
            ..Default::default()
        };
        let ContainerCreateResponse { id, .. } = self
            .docker
            .create_container::<&str, String>(
                Some(CreateContainerOptions {
                    name: &self.container_name,
                }),
                config,
            )
            .await?;
        self.docker.start_container::<String>(&id, None).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use bitcoin::Network;

    use super::*;

    #[test]
    fn test_start_stop_bitcoind() -> Result<(), Error> {
        let rpc_config = RpcConfig {
            username: "foo".to_string(),
            password: "rpcpassword".to_string(),
            url: "http://localhost:18443".to_string(),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let bitcoind = Bitcoind::new(
            "bitcoin-regtest",
            "ruimarinho/bitcoin-core",
            rpc_config.clone(),
        );

        bitcoind.start()?;
        bitcoind.stop()?;

        Ok(())
    }

    #[test]
    fn test_start_stop_bitcoind_with_flags() -> Result<(), Error> {
        let rpc_config = RpcConfig {
            username: "foo".to_string(),
            password: "rpcpassword".to_string(),
            url: "http://localhost:18443".to_string(),
            wallet: "mywallet".to_string(),
            network: Network::Regtest,
        };

        let flags = BitcoindFlags {
            min_relay_tx_fee: 0.00001,
            block_min_tx_fee: 0.00001,
            debug: 1,
            fallback_fee: 0.0002,
        };

        let bitcoind = Bitcoind::new_with_flags(
            "bitcoin-regtest",
            "ruimarinho/bitcoin-core",
            rpc_config.clone(),
            flags,
        );

        bitcoind.start()?;
        bitcoind.stop()?;

        Ok(())
    }
}
