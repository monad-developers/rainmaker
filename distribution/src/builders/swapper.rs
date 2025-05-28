use crate::prelude::*;

sol! {
    contract Swapper {
        function swap(uint256 amountIn, bool aToB) external;
    }
}
pub struct SwapperBuilder {
    pub signer: PrivateKeySigner,
    pub provider: Box<dyn Provider>,
    pub contract_address: Address,
    pub max_swaps: u64,
}

#[async_trait]
impl Builder for SwapperBuilder {
    async fn build_transactions(&self, _start_nonce_override: Option<u64>) -> Result<Vec<String>> {
        let starting_nonce = self
            .provider
            .get_transaction_count(self.signer.address())
            .await?;
        let gas_limit = {
            let sim_tx = TransactionRequest::default()
                .from(self.signer.address())
                .to(self.contract_address)
                .input(
                    Swapper::swapCall {
                        amountIn: U256::from(100),
                        aToB: false,
                    }
                    .abi_encode()
                    .into(),
                );
            self.provider.estimate_gas(&sim_tx).await? * 105 / 100
        };

        Ok((0..self.max_swaps)
            .into_par_iter()
            .map(|i| {
                let mut tx = TxLegacy::default();
                tx.nonce = starting_nonce + i;
                tx.gas_price = 52_000_000_000u128;
                tx.gas_limit = gas_limit;
                tx.to = TxKind::Call(self.contract_address);
                tx.value = U256::ZERO;

                // Encode swap call
                let call = Swapper::swapCall {
                    amountIn: U256::from(100),
                    aToB: i % 2 == 0,
                };
                tx.input = call.abi_encode().into();

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
            .collect::<Vec<_>>())
    }
}
