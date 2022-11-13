use std::collections::HashMap;

use ethers::types::{Address, Bytes, H256, U256};
use reqwest::Url;
use serde::Serialize;

use crate::{client::ClientResult, SafeClient};

use super::common::{DecimalU256, Operations};

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct SafeMultiSigTxRequest;

impl SafeMultiSigTxRequest {
    pub fn url(root: &Url, tx: H256) -> Url {
        let path = format!("api/v1/multisig-transactions/{:?}/", tx);
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeMultisigConfirmationResponse {
    pub owner: Address,
    /// Date at which the confirmation was submitted
    pub submission_date: String,
    pub transaction_hash: Option<H256>,
    pub signature: String,      // RSV
    pub signature_type: String, // this should be an enum?
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeMultisigTransactionResponse {
    /// Address of the safe
    pub safe: Address,
    /// Target of the transaction
    pub to: Address,
    /// Native asset value included in the transaction
    #[serde(default)]
    pub value: DecimalU256,
    /// Data payload sent to target by safe
    #[serde(default)]
    pub data: Bytes,
    /// CALL or DELEGATECALL
    pub operation: Operations,
    /// token used to refund gas, address(0) for native asset
    pub gas_token: Address,
    /// Refundable gas that can be used by the safe for sig checking & admin
    pub safe_tx_gas: u64,
    pub base_gas: u64,
    pub gas_price: DecimalU256,
    /// Address to which to issue gas refunds
    pub refund_receiver: Address,
    /// Tx Nonce
    pub nonce: u64,
    /// Execution time, if executed
    pub execution_date: Option<String>,
    /// Time tx was submitted to the safe transaction service
    pub submission_date: String,
    /// Time tx was modified
    pub modified: String,
    /// Block number of confirmation (none if unconfirmed)
    #[serde(default)]
    pub block_number: Option<u32>,
    /// Transaction hash of confirmation (if any)
    #[serde(default)]
    pub transaction_hash: Option<H256>,
    /// Safe internal tx hash, produced by EIP712
    pub safe_tx_hash: H256,
    /// Address of account that executed this safe tx (if executed)
    #[serde(default)]
    pub executor: Option<Address>,
    /// Execution status. `true` if executed, `false` otherwise
    pub is_executed: bool,
    /// Success status. `None` if tx is not executed, else `true` if
    /// successful, `false` if revert
    #[serde(default)]
    pub is_successful: Option<bool>,
    #[serde(default)]
    pub eth_gas_price: Option<U256>,
    #[serde(default)]
    pub max_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub gas_used: Option<u32>,
    #[serde(default)]
    pub fee: Option<u64>,
    #[serde(default)]
    pub origin: Option<Address>, // is this correct?
    #[serde(default)]
    pub data_decoded: Option<String>,
    #[serde(default)]
    pub confirmations_required: Option<u32>,
    /// Confirmations of the transaction by owners
    pub confirmations: Vec<SafeMultisigConfirmationResponse>,
    pub trusted: bool,
    pub signatures: Option<String>, // RSV string
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsigHistoryResponse {
    pub count: u64,
    #[serde(default)]
    pub next: Option<Url>,
    #[serde(default)]
    pub previous: Option<Url>,
    pub results: Vec<SafeMultisigTransactionResponse>,
}

#[derive(serde::Serialize, Clone)]
pub struct MsigHistoryRequest<'a> {
    #[serde(flatten)]
    filters: HashMap<&'static str, String>,
    #[serde(skip)]
    client: &'a SafeClient,
}

impl<'a> MsigHistoryRequest<'a> {
    // TODO: `modified` filters
    // TODO: Execution date & submission date

    // deliberately not supporting LT and GT. redundant
    const NONCE_KEYS: &'static [&'static str] = &["nonce__gte", "nonce__lte", "nonce"];

    // GT and GTE not supported by API for some reason
    const VALUE_KEYS: &'static [&'static str] = &["value__gt", "value__lte", "value"];

    /// Insert a KV pair into the internal mapping for later URL encoding
    ///
    /// Somewhat more expensive and brittle than required, as it uses
    /// serde_json. Using display would cause hashes and addresses to be
    /// abbreviated `0xabcd....1234`
    fn insert<S: Serialize>(&mut self, k: &'static str, v: S) {
        self.filters.insert(k, serde_json::to_string(&v).unwrap());
    }

    pub fn url(root: &Url, address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/multisig-transactions/",
            ethers::utils::to_checksum(&address, None)
        );
        let mut url = root.clone();
        url.set_path(&path);
        url
    }

    pub fn new(client: &'a SafeClient) -> Self {
        Self {
            filters: Default::default(),
            client,
        }
    }

    pub async fn query(self, address: Address) -> ClientResult<MsigHistoryResponse> {
        self.client
            .filtered_msig_history(address, &self.filters)
            .await
    }

    fn clear_nonces(&mut self) {
        Self::NONCE_KEYS.iter().for_each(|k| {
            self.filters.remove(k);
        });
    }

    fn clear_values(&mut self) {
        Self::VALUE_KEYS.iter().for_each(|k| {
            self.filters.remove(k);
        });
    }

    pub fn min_nonce(mut self, min_nonce: u32) -> Self {
        self.filters.remove("nonce");
        self.insert("nonce__gte", min_nonce);
        self
    }

    pub fn max_nonce(mut self, max_nonce: u32) -> Self {
        self.filters.remove("nonce");
        self.insert("nonce__lte", max_nonce);
        self
    }

    pub fn nonce(mut self, nonce: u32) -> Self {
        self.clear_nonces();
        self.insert("nonce", nonce);
        self
    }

    pub fn safe_tx_hash(mut self, h: impl Into<H256>) -> Self {
        self.insert::<H256>("safe_tx_hash", h.into());
        self
    }

    pub fn to(mut self, addr: Address) -> Self {
        self.insert("safe_tx_hash", addr);
        self
    }

    pub fn min_value(mut self, value: u64) -> Self {
        self.filters.remove("value");
        self.insert("value__gt", value.saturating_sub(1));
        self
    }

    pub fn max_value(mut self, value: u64) -> Self {
        self.filters.remove("value");
        self.insert("value__lt", value.saturating_add(1));
        self
    }

    pub fn value(mut self, value: u64) -> Self {
        self.clear_values();
        self.insert("value", value);
        self
    }

    pub fn executed(mut self, executed: &str) -> Self {
        self.insert("executed", executed.to_owned());
        self
    }

    pub fn trusted(mut self, trusted: &str) -> Self {
        self.insert("trusted", trusted.to_owned());
        self
    }

    pub fn transaction_hash(mut self, transaction_hash: impl Into<H256>) -> Self {
        self.insert("transaction_hash", transaction_hash.into());
        self
    }

    pub fn ordering(mut self, ordering: &str) -> Self {
        self.insert("ordering", ordering.to_owned());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.insert("limit", limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.insert("offset", offset);
        self
    }
}

#[cfg(test)]
mod test {
    use super::SafeMultisigTransactionResponse;

    #[test]
    fn it_parses() {
        let resp = "{\"safe\":\"0x38CD8Fa77ECEB4b1edB856Ed27aac6A6c6Dc88ca\",\"to\":\"0xD5F586B9b2abbbb9a9ffF936690A54F9849dbC97\",\"value\":\"381832418\",\"data\":\"0xdeadbeefdeadbeef\",\"operation\":1,\"gasToken\":\"0x0000000000000000000000000000000000000000\",\"safeTxGas\":0,\"baseGas\":0,\"gasPrice\":\"0\",\"refundReceiver\":\"0x0000000000000000000000000000000000000000\",\"nonce\":0,\"executionDate\":null,\"submissionDate\":\"2022-11-13T18:16:49.292148Z\",\"modified\":\"2022-11-13T18:16:49.325143Z\",\"blockNumber\":null,\"transactionHash\":null,\"safeTxHash\":\"0xa13429644bc3e3871867f1b6f48b092e397b8cc582cdd48504c24a3d445ace9e\",\"executor\":null,\"isExecuted\":false,\"isSuccessful\":null,\"ethGasPrice\":null,\"maxFeePerGas\":null,\"maxPriorityFeePerGas\":null,\"gasUsed\":null,\"fee\":null,\"origin\":null,\"dataDecoded\":null,\"confirmationsRequired\":null,\"confirmations\":[{\"owner\":\"0xD5F586B9b2abbbb9a9ffF936690A54F9849dbC97\",\"submissionDate\":\"2022-11-13T18:16:49.325143Z\",\"transactionHash\":null,\"signature\":\"0x25ca0eaef716dffb7ad380cb428be2413f2db5f5131b6694801383ebabe22a1314ed82db6a4cc9b3d13d0b318e1ef57a8f4d9690b58bc9db1ac4806e7bb8f0191b\",\"signatureType\":\"EOA\"}],\"trusted\":true,\"signatures\":null}";

        serde_json::from_str::<SafeMultisigTransactionResponse>(&resp).unwrap();
    }
}
