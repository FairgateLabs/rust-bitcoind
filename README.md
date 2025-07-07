# Rust Bitcoin Docker

A Rust library for managing a Bitcoin Core node in a Docker container.

## Features
- Start and manage Bitcoin Core nodes in regtest mode
- Docker container lifecycle management (create, start, stop, cleanup)
- Error handling and container state management
- Automatic image pulling
- Configurable container and Bitcoin Core settings



## Methods

The `Bitcoind` struct provides several methods to manage a Bitcoin Core node within a Docker container:

- **new**
  Creates a new `Bitcoind` instance with default flags.

- **new_with_flags**
  Creates a new `Bitcoind` instance with specified flags.

- **start**
  Starts the `Bitcoind` Docker container. It checks if the Docker daemon is active and attempts to start the container. If the image is not found, it will pull the image and retry.

- **stop**
  Stops the `Bitcoind` Docker container.

 









