use crate::prelude::*;

sol! {
    contract BatchSender {
        receive() external payable {}
        function batchSend(address[] calldata recipients, uint256[] calldata amounts) external payable;
    }
}

fn float_to_u256(amount: f64) -> U256 {
    // Create a U256 representation of 10^18 (1 ETH in wei)
    let one_eth_in_wei = U256::from(10).pow(U256::from(18));

    // Convert the float to a fixed-point representation
    // First, get the integer part
    let integer_part = amount.floor() as u64;

    // Then get the fractional part with 18 decimals of precision
    let fractional_part = ((amount - integer_part as f64) * 1e18) as u64;

    // Combine both parts
    U256::from(integer_part) * one_eth_in_wei + U256::from(fractional_part)
}

// Native EOA transfer builder
pub struct NativeTransferBuilder {
    pub provider: Box<dyn Provider>,
    pub signer: PrivateKeySigner,
    pub recipients: Vec<Address>,
    pub amounts: Vec<f64>,
}

#[async_trait]
impl Builder for NativeTransferBuilder {
    async fn build_transactions(&self, start_nonce_override: Option<u64>) -> Result<Vec<String>> {
        assert!(
            self.recipients.len() == self.amounts.len(),
            "Recipients and amounts must be the same length"
        );
        let gas_limit = 21_000u64;
        let starting_nonce = if let Some(start_nonce) = start_nonce_override {
            start_nonce
        } else {
            self.provider
                .get_transaction_count(self.signer.address())
                .await?
        };
        Ok(self
            .recipients
            .par_iter()
            .enumerate()
            .map(|(i, recipient)| {
                let mut tx = TxLegacy::default();
                tx.nonce = starting_nonce + i as u64;
                tx.gas_price = 52_000_000_000u128;
                tx.gas_limit = gas_limit;
                tx.to = TxKind::Call(*recipient);
                tx.value = float_to_u256(self.amounts[i]);
                tx.input = vec![].into();
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

// Batch sender contract builder
pub struct NativeBatchSenderBuilder {
    pub signer: PrivateKeySigner,
    pub provider: Box<dyn Provider>,
    pub contract_address: Address,
    pub recipients: Vec<Address>,
    pub amounts: Vec<f64>,
}

#[async_trait]
impl Builder for NativeBatchSenderBuilder {
    async fn build_transactions(&self, start_nonce_override: Option<u64>) -> Result<Vec<String>> {
        assert!(
            self.recipients.len() == self.amounts.len(),
            "Recipients and amounts must be the same length"
        );
        let starting_nonce = if let Some(start_nonce) = start_nonce_override {
            start_nonce
        } else {
            self.provider
                .get_transaction_count(self.signer.address())
                .await?
        };
        let batch_addresses = self.recipients.chunks(1000).collect::<Vec<_>>();
        let batch_amounts = self
            .amounts
            .clone()
            .into_iter()
            .map(float_to_u256)
            .collect::<Vec<_>>();
        let batch_amounts = batch_amounts.chunks(1000).collect::<Vec<_>>();

        let mut gas_limits = Vec::new();
        info!("Estimating batch gas limits...");
        for i in 0..batch_addresses.len() {
            info!("---> Estimating gas for batch {}", i);
            let batch_transfer_call = BatchSender::batchSendCall {
                recipients: batch_addresses[i].to_vec(),
                amounts: batch_amounts[i].to_vec(),
            };
            let calldata = batch_transfer_call.abi_encode();
            let sim_tx = TransactionRequest::default()
                .from(self.signer.address())
                .to(self.contract_address)
                .value(batch_amounts[i].iter().sum())
                .input(calldata.into());
            gas_limits.push(self.provider.estimate_gas(&sim_tx).await? * 110 / 100);
        }
        info!("Batch gas limits estimated");

        Ok(batch_addresses
            .into_par_iter()
            .enumerate()
            .map(|(i, recipients)| {
                let amounts = batch_amounts[i].to_vec();
                let gas_limit = gas_limits[i];

                let mut tx = TxLegacy::default();
                tx.nonce = starting_nonce + i as u64;
                tx.gas_price = 52_000_000_000u128;
                tx.gas_limit = gas_limit;
                tx.to = TxKind::Call(self.contract_address);
                tx.value = amounts.iter().sum();

                let batch_transfer_call = BatchSender::batchSendCall {
                    recipients: recipients.to_vec(),
                    amounts,
                };
                let calldata = batch_transfer_call.abi_encode();
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
