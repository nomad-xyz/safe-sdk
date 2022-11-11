use std::collections::HashMap;

use ethers::types::{Address, Bytes, H256, U256};
use reqwest::Url;
use serde::Serialize;

use crate::{client::ClientResult, SafeClient};

use super::common::Operations;

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeMultisigConfirmationResponse {
    pub owner: Address,
    pub submission_date: String,
    pub transaction_hash: H256,
    pub signature: String,      // is this RSV? VSR?
    pub signature_type: String, // this should be an enum?
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeMultisigTransactionResponse {
    pub safe: Address,
    pub to: Address,
    #[serde(default)]
    pub value: U256,
    #[serde(default)]
    pub data: Bytes,
    pub operation: Operations,
    pub gas_token: Option<Address>,
    pub safe_tx_gas: u64,
    pub base_gas: u64,
    pub gas_price: U256,
    pub refund_receiver: Address,
    pub nonce: u32,
    pub execution_date: String,
    pub submission_date: String,
    pub modified: String,
    pub block_number: u32,
    pub transaction_hash: H256,
    pub safe_tx_hash: H256,
    pub executor: Address,
    pub is_executed: bool,
    pub is_successful: bool,
    pub eth_gas_price: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub gas_used: u32,
    pub fee: u64,
    pub origin: Address, // is this correct?
    pub data_decoded: String,
    pub confirmations_required: u32,
    pub confirmations: SafeMultisigConfirmationResponse,
    pub trusted: bool,
    pub signatures: String, // is this RSV? VSR?
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsigHistoryResponse {
    pub count: u32,
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
