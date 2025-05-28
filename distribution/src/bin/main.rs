use std::collections::HashSet;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;

use clap::{App, Arg};
use tracing::{error, info, warn};
use tracing_subscriber::{self, EnvFilter};

use distribution::builders;
use distribution::distributors;
use distribution::prelude::*;

// Helper to read the addresses file
pub fn read_addresses_file(filename: &str) -> io::Result<Vec<(Address, Option<f64>)>> {
    let path = Path::new(filename);
    let file = File::open(path)?;
    let lines = io::BufReader::new(file).lines();

    let mut address_amounts = Vec::new();
    let mut unique_addresses = HashSet::new();
    let mut has_amounts = false;

    for line in lines {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        if let Ok(address) = parts[0].trim().parse::<Address>() {
            if unique_addresses.insert(address) {
                address_amounts.push((
                    address,
                    if parts.len() > 1 {
                        has_amounts = true;
                        Some(parts[1].parse::<f64>().unwrap())
                    } else {
                        if has_amounts {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Missing amounts for some addresses",
                            ));
                        }
                        None
                    },
                ));
            } else {
                warn!("Skipping duplicate address: {}", address);
            }
        } else {
            warn!("Skipping invalid address: {}", parts[0]);
        }
    }

    Ok(address_amounts)
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    dotenv().ok();

    // Parse command-line arguments
    let matches = App::new("Duster")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("config.yml")
                .takes_value(true),
        )
        .get_matches();

    // Read config file
    let config_file = matches
        .value_of("config")
        .expect("Config file must be specified");
    let config = read_config_file::<UnifiedConfig>(config_file);

    // Validate that we have at least one RPC URL
    if config.core.rpc_urls.is_empty() {
        error!("At least one RPC URL must be specified in config");
        std::process::exit(1);
    }

    // Create distribution options
    let options = DistributionOptions {
        rpc_urls: config.core.rpc_urls.clone(),
        rpc_batch_size: config.core.rpc_batch_size,
        target_tps: config.core.target_tps,
        wait_for_confirmation: true,
        confirmation_timeout: 120,
    };

    // Get private key from environment
    let signer = {
        let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
        let private_bytes = hex::decode(&private_key[2..]).unwrap();
        PrivateKeySigner::from_bytes(&FixedBytes::from_slice(&private_bytes)).unwrap()
    };

    // Create provider for the chain (using first RPC URL for builder operations)
    let provider = ProviderBuilder::new().on_http(config.core.rpc_urls[0].parse().unwrap());

    // Extract and prepare addresses and amounts
    let (addresses, amounts) = match config.core.distribution_type {
        DistributionType::NativeDirect | DistributionType::NativeBatch => {
            let addresses_file = config.core.addresses_file.as_ref().expect(&format!(
                "Addresses file must be specified for distribution type {:?}",
                config.core.distribution_type
            ));
            let address_data = read_addresses_file(&addresses_file)?;
            // Check if we have amounts from file
            let have_amounts_from_file = address_data[0].1.is_some();

            if have_amounts_from_file {
                // Use file amounts
                let addresses = address_data
                    .iter()
                    .map(|(addr, _)| *addr)
                    .collect::<Vec<_>>();
                let amounts = address_data
                    .iter()
                    .map(|(_, amt)| amt.expect("Amount must be set in file"))
                    .collect::<Vec<_>>();
                (addresses, amounts)
            } else {
                // Use config amounts
                let token_config = config.token.as_ref().expect(&format!(
                    "Token config must be set for distribution type {:?}",
                    config.core.distribution_type
                ));
                let addresses = address_data
                    .iter()
                    .map(|(addr, _)| *addr)
                    .collect::<Vec<_>>();
                let amounts = if token_config.amount_per_address_low
                    == token_config.amount_per_address_high
                {
                    // Fixed amount
                    vec![token_config.amount_per_address_low; addresses.len()]
                } else {
                    // Random amount in range
                    let mut rng = rand::rng();
                    addresses
                        .iter()
                        .map(|_| {
                            rng.random_range(
                                token_config.amount_per_address_low
                                    ..=token_config.amount_per_address_high,
                            )
                        })
                        .collect()
                };

                (addresses, amounts)
            }
        }
        DistributionType::NFTMint => {
            let addresses_file = config.core.addresses_file.as_ref().expect(&format!(
                "Addresses file must be specified for distribution type {:?}",
                config.core.distribution_type
            ));
            let address_data = read_addresses_file(&addresses_file)?;
            let addresses = address_data
                .iter()
                .map(|(addr, _)| *addr)
                .collect::<Vec<_>>();
            (addresses, vec![])
        }
        _ => (vec![], vec![]),
    };

    {
        info!("Distribution type: {:?}", config.core.distribution_type);
        info!("Signer address: {:?}", signer.address());
        info!("Total addresses: {}", addresses.len());
        info!("Total amount: {}", amounts.iter().sum::<f64>());
        info!("RPC URLs: {:?}", config.core.rpc_urls);
        info!("RPC batch size: {}", options.rpc_batch_size);
        info!("Target TPS: {}", options.target_tps);

        info!("\nPress Enter to continue or Ctrl+C to cancel...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim() != "" {
            error!("Aborted");
            std::process::exit(1);
        }
    }

    // Create appropriate builder based on distribution type
    let builder: Box<dyn Builder + Send + Sync>;
    let distributor: Box<dyn Distributor>;
    match config.core.distribution_type {
        DistributionType::NativeDirect => {
            builder = Box::new(builders::native::NativeTransferBuilder {
                signer: signer.clone(),
                provider: Box::new(provider.clone()),
                recipients: addresses,
                amounts,
            });
            distributor = Box::new(distributors::continuous::ContinuousDistributor { signer });
        }
        DistributionType::NativeBatch => {
            builder = Box::new(builders::native::NativeBatchSenderBuilder {
                signer: signer.clone(),
                provider: Box::new(provider.clone()),
                contract_address: config.token.unwrap().batch_sender_address,
                recipients: addresses,
                amounts,
            });
            distributor = Box::new(distributors::finite::FiniteGroupDistributor { signer });
        }
        DistributionType::NFTMint => {
            let nft_config = config.nft.expect(&format!(
                "NFT config must be set for distribution type {:?}",
                config.core.distribution_type
            ));
            if nft_config.soulbound {
                builder = Box::new(builders::nft::SoulboundNFTMintBuilder {
                    signer: signer.clone(),
                    provider: Box::new(provider.clone()),
                    contract_address: nft_config.token_address,
                    recipients: addresses,
                    image_url: nft_config.image_url,
                })
            } else {
                unimplemented!("Non-soulbound NFT minting is not implemented yet");
            }
            distributor = Box::new(distributors::finite::FiniteGroupDistributor { signer });
        }
        DistributionType::Swapper => {
            builder = Box::new(builders::swapper::SwapperBuilder {
                signer: signer.clone(),
                provider: Box::new(provider.clone()),
                contract_address: config
                    .swapper
                    .as_ref()
                    .expect("Swapper config must be set")
                    .swapper_address,
                max_swaps: config.swapper.as_ref().unwrap().max_swaps,
            });
            distributor = Box::new(distributors::continuous::ContinuousDistributor { signer });
        }
    };

    distributor
        .send_transactions(config.core.rpc_urls, builder, options)
        .await?;

    info!("Distribution completed successfully!");
    Ok(())
}
