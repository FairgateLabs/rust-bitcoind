# Rust Bitcoin Docker

A Rust library for managing a Bitcoin Core node in a Docker container.

## ⚠️ Disclaimer

This library is currently under development and may not be fully stable.
It is not production-ready, has not been audited, and future updates may introduce breaking changes without preserving backward compatibility.

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
// Use the constructor: the struct's fields are `Secret`-wrapped, and the network
// argument accepts a `bitcoin::Network` or a `NetworkFlavor`.
let rpc_config = RpcConfig::new(
    Network::Regtest,
    "http://localhost:18443".to_string(),
    "bitcoin".to_string(),
    "password".to_string(),
    "default".to_string(),
);

// Create a new Bitcoin Core instance
let bitcoind = Bitcoind::new("my-bitcoin-node", "bitcoin/bitcoin:29.1", rpc_config)?;

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

// Use the constructor: the struct's fields are `Secret`-wrapped, and the network
// argument accepts a `bitcoin::Network` or a `NetworkFlavor`.
let rpc_config = RpcConfig::new(
    Network::Regtest,
    "http://localhost:18443".to_string(),
    "bitcoin".to_string(),
    "password".to_string(),
    "default".to_string(),
);

let flags = BitcoindFlags {
    min_relay_tx_fee: 0.00003,
    block_min_tx_fee: 0.00004,
    debug: 0,
    fallback_fee: 0.0002,
};

let bitcoind = Bitcoind::new_with_flags("my-node", "bitcoin/bitcoin:29.1", rpc_config, flags);
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

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.

---

## 🧩 Part of the BitVMX Ecosystem

This repository is a component of the **BitVMX Ecosystem**, an open platform for disputable computation secured by Bitcoin.  
You can find the index of all BitVMX open-source components at [**FairgateLabs/BitVMX**](https://github.com/FairgateLabs/BitVMX).

---
