use crate::prelude::*;

sol! {
    contract SoulboundMinter {
        function batchMint(address[] calldata recipients, string memory uri) external;
    }
}

pub struct SoulboundNFTMintBuilder {
    pub signer: PrivateKeySigner,
    pub provider: Box<dyn Provider>,
    pub contract_address: Address,
    pub recipients: Vec<Address>,
    pub image_url: String,
}

#[async_trait]
impl Builder for SoulboundNFTMintBuilder {
    async fn build_transactions(&self, _start_nonce_override: Option<u64>) -> Result<Vec<String>> {
        let starting_nonce = self
            .provider
            .get_transaction_count(self.signer.address())
            .await?;
        let batch_addresses = self.recipients.chunks(500).collect::<Vec<_>>();

        let mut gas_limits = Vec::new();
        info!("Estimating batch gas limits...");
        for i in 0..batch_addresses.len() {
            info!("---> Estimating gas for batch {}", i);
            let batch_mint_call = SoulboundMinter::batchMintCall {
                recipients: batch_addresses[i].to_vec(),
                uri: self.image_url.clone(),
            };
            let calldata = batch_mint_call.abi_encode();
            let sim_tx = TransactionRequest::default()
                .from(self.signer.address())
                .to(self.contract_address)
                .input(calldata.into());
            gas_limits.push(self.provider.estimate_gas(&sim_tx).await? * 105 / 100);
        }
        info!("Batch gas limits estimated");

        Ok(batch_addresses
            .into_par_iter()
            .enumerate()
            .map(|(i, recipients)| {
                let gas_limit = gas_limits[i];

                let mut tx = TxLegacy::default();
                tx.nonce = starting_nonce + i as u64;
                tx.gas_price = 52_000_000_000u128;
                tx.gas_limit = gas_limit;
                tx.to = TxKind::Call(self.contract_address);
                tx.value = U256::from(0);

                let batch_mint_call = SoulboundMinter::batchMintCall {
                    recipients: recipients.to_vec(),
                    uri: self.image_url.clone(),
                };
                let calldata = batch_mint_call.abi_encode();
                tx.input = calldata.into();

                tx.chain_id = Some(10143);

                let signature = self
                    .signer
                    .sign_transaction_sync(&mut tx)
                    .expect("Failed to sign transaction");
                let signed_tx = tx.into_signed(signature);

                let mut buf = Vec::new();
                signed_tx.rlp_encode(&mut buf);
                format!("0x{}", hex::encode(buf))
            })
            .collect())
    }
}
