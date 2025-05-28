use crate::prelude::*;
use itertools::Itertools;
use tokio::task::JoinSet;

pub struct ContinuousDistributor {
    pub signer: PrivateKeySigner,
}

#[async_trait]
impl Distributor for ContinuousDistributor {
    async fn send_transactions(
        &self,
        rpc_urls: Vec<String>,
        builder: Box<dyn Builder + Send + Sync>,
        options: DistributionOptions,
    ) -> Result<()> {
        // Use the first RPC URL to create a provider for nonce management
        let provider = ProviderBuilder::new().on_http(rpc_urls[0].parse().unwrap());
        let mut base_nonce = provider
            .get_transaction_count(self.signer.address())
            .await
            .unwrap();
        info!("Base nonce: {}", base_nonce);
        
        let delay_increment_ms = {
            let requests_per_sec = options.target_tps as f64 / options.rpc_batch_size as f64;
            (1000.0 / requests_per_sec) as u64
        };
        let http_client = Client::new();

        loop {
            info!("Signing transactions...");
            let all_txs = builder.build_transactions(Some(base_nonce)).await?;
            base_nonce += all_txs.len() as u64;

            info!("Sending batches across {} RPC endpoints...", rpc_urls.len());
            let mut delay_ms = 0u64;
            let mut join_set = JoinSet::new();
            for (i, chunk) in all_txs
                .into_iter()
                .chunks(options.rpc_batch_size)
                .into_iter()
                .enumerate()
            {
                let txs = chunk.collect::<Vec<_>>();
                let client = http_client.clone();
                let url = &rpc_urls[i % rpc_urls.len()];
                join_set.spawn(send_transactions_with_delay(
                    txs,
                    client,
                    url.clone(),
                    delay_ms,
                ));
                delay_ms += delay_increment_ms;
            }

            info!("Awaiting batches to be dispatched");
            let _ = join_set.join_all().await;
            info!("All batches dispatched successfully");
        }
    }
}
