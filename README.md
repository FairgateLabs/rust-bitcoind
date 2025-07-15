# Rust Bitcoin Docker

A Rust library for managing a Bitcoin Core node in a Docker container.

## Features
- 🚀 Start and manage Bitcoin Core nodes in regtest mode
- 🐳 Docker container lifecycle management (create, start, stop, cleanup)
- ⚠️ Error handling and container state management
- 📥 Automatic image pulling
- ⚙️ Configurable container and Bitcoin Core settings

## Usage

### Basic Usage

```rust
use bitcoind::{Bitcoind, BitcoindFlags};
use bitvmx_bitcoin_rpc::rpc_config::RpcConfig;
use bitcoin::Network;

// Configure RPC settings
let rpc_config = RpcConfig {
    username: "bitcoin".to_string(),
    password: "password".to_string(),
    url: "http://localhost:18443".to_string(),
    wallet: "default".to_string(),
    network: Network::Regtest,
};

// Create a new Bitcoin Core instance
let bitcoind = Bitcoind::new("my-bitcoin-node", "ruimarinho/bitcoin-core", rpc_config)?;

// Start the container
bitcoind.start()?;

// Your Bitcoin operations here...
println!("Bitcoin Core node is running!");

// Stop the container
bitcoind.stop()?;

```

### Custom Configuration

```rust
use bitcoind::{Bitcoind, BitcoindFlags};
use bitvmx_bitcoin_rpc::rpc_config::RpcConfig;
use bitcoin::Network;
use std::time::Duration;

let rpc_config = RpcConfig {
    username: "bitcoin".to_string(),
    password: "password".to_string(),
    url: "http://localhost:18443".to_string(),
    wallet: "default".to_string(),
    network: Network::Regtest,
};

let flags = BitcoindFlags {
    min_relay_tx_fee: 0.00003,
    block_min_tx_fee: 0.00004,
    debug: 0,
    fallback_fee: 0.0002,
};

let bitcoind = Bitcoind::new_with_flags("my-node", "ruimarinho/bitcoin-core", rpc_config, flags);
```

### Bitcoind Flags

| Field | Description | Default |
|-------|-------------|---------|
| `min_relay_tx_fee` | Minimum relay transaction fee (in BTC) | `0.00001` |
| `block_min_tx_fee` | Minimum transaction fee for block inclusion (in BTC) | `0.00001` |
| `debug` | Debug level | `1` |
| `fallback_fee` | Fallback fee (in BTC) | `0.0002` |

### Development Setup

1. Clone the repository
2. Install dependencies: `cargo build`
3. Run tests: `cargo test -- --test-threads=1`

## Contributing
Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License
This project is licensed under the MIT License.

