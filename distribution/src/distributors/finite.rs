use crate::prelude::*;
use tokio::task::JoinSet;

sol! {
    contract BatchSender {
        receive() external payable {}
        function batchSend(address[] calldata recipients, uint256[] calldata amounts) external payable;
    }
}

pub struct FiniteGroupDistributor {
    pub signer: PrivateKeySigner,
}

#[async_trait]
impl Distributor for FiniteGroupDistributor {
    async fn send_transactions(
        &self,
        rpc_urls: Vec<String>,
        builder: Box<dyn Builder + Send + Sync>,
        options: DistributionOptions,
    ) -> Result<()> {
        let delay_increment_ms = {
            let requests_per_sec = options.target_tps as f64 / options.rpc_batch_size as f64;
            (1000.0 / requests_per_sec) as u64
        };
        let http_client = Client::new();

        info!("Signing transactions...");
        let all_txs = builder.build_transactions(None).await?;

        info!("Sending {} transactions across {} RPC endpoints...", all_txs.len(), rpc_urls.len());
        let mut delay_ms = 0u64;
        let mut join_set = JoinSet::new();
        let mut group_number = 1;

        for (i, tx) in all_txs.iter().enumerate() {
            let client = http_client.clone();
            let url = &rpc_urls[i % rpc_urls.len()];
            join_set.spawn(send_transactions_with_delay(
                vec![tx.clone()],
                client,
                url.clone(),
                delay_ms,
            ));
            delay_ms += delay_increment_ms;

            if i % 10 == 0 || i == all_txs.len() - 1 {
                info!("Waiting for group {} to be dispatched", group_number);
                let _ = join_set.join_all().await;
                info!("Group {} dispatched successfully", group_number);
                delay_ms = 0;
                join_set = JoinSet::new();
                group_number += 1;
            }
        }

        info!("All groups dispatched successfully");
        Ok(())
    }
}
