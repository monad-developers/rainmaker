# Rainmaker

A Rust tool for distributing native tokens, minting NFTs, and executing swaps with configurable throughput

## Quick Start

1. Set your private key:
   ```bash
   export PRIVATE_KEY=0x...
   ```

2. Configure `config.yml`:
   ```yaml
   core:
     rpc_urls: 
      - "https://testnet-rpc.monad.xyz"
     target_tps: 500
     rpc_batch_size: 100
     distribution_type: "native-direct"
     addresses_file: "addresses.txt"
   ```

3. Create `addresses.txt` with recipient addresses:
   ```
   0x1234567890123456789012345678901234567890
   0xabcdefabcdefabcdefabcdefabcdefabcdefabcd
   ```

4. Run:
   ```bash
   cd distribution/
   cargo run --release
   ```

## Configuration

### Core Settings (Required)
```yaml
core:
  rpc_urls:                                    # RPC endpoints (round robin)
      - "https://testnet-rpc.monad.xyz"
  target_tps: 2000                             # Transactions per second
  rpc_batch_size: 100                          # Transactions per RPC batch call
  distribution_type: "native-direct"           # Distribution type
  addresses_file: "addresses.txt"              # Address list file
```

**Distribution Types:**
- `native-direct` - Direct native token transfers
- `native-batch` - Batched native token transfers (via batch sender contract)
- `nft-mint` - NFT minting (via batch minting function)
- `swapper` - Uniswap V2 swaps (continuous)

### Token Settings (Optional)
For native token distributions:
```yaml
token:
  amount_per_address_low: 0.00001               # Min amount per address
  amount_per_address_high: 0.00002              # Max amount per address
  batch_sender_address: "0x..."                 # Batch sender contract address (optional)
```

### NFT Settings (Optional)
For NFT minting:
```yaml
nft:
  token_address: "0x..."                      # NFT contract address
  soulbound: true                             # Whether the NFTs are soulbound
  image_url: "https://example.com/nft.json"   # Metadata URL
```

### Swapper Settings (Optional)
For swapper interactions:
```yaml
swapper:
  swapper_address: "0x..."                    # Swapper contract address
  max_swaps: 10000                            # Maximum swaps to execute
```

## Address File Format

Simple format (one address per line):
```
0x1234567890123456789012345678901234567890
0xabcdefabcdefabcdefabcdefabcdefabcdefabcd
```

CSV format with amounts (overrides config amounts):
```
0x1234567890123456789012345678901234567890,0.001
0xabcdefabcdefabcdefabcdefabcdefabcdefabcd,0.002
```

## Command Line Options

```bash
cd distribution/
cargo run --release -- [OPTIONS]

-c, --config <FILE>        Config file [default: config.yml]
```

## How It Works

The tool batches transactions to optimize RPC usage and achieve target TPS:

```
delay_between_batches = (batch_size / target_tps) * 1000ms
```

This approach:
- Reduces RPC overhead
- Prevents overwhelming the RPC node
- Maintains consistent transaction timing
- Minimizes nonce conflicts 