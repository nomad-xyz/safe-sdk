use std::collections::HashMap;

use async_stream::stream;
use ethers::types::{Address, Bytes, H256, U256};
use reqwest::Url;
use serde::Serialize;

use crate::{client::ClientResult, SafeClient};

use super::common::{Operations, Paginated};

/// Response for multisig history requests
pub type MsigHistoryResponse = Paginated<MsigTxResponse>;

/// Safe Multisig history request
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MsigTxRequest;

impl MsigTxRequest {
    /// The URL to which to dispatch this request
    pub fn url(root: &Url, tx: H256) -> Url {
        let path = format!("api/v1/multisig-transactions/{:?}/", tx);
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

/// Decoded function call parameter
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Solidity type of parameter
    #[serde(rename = "type")]
    pub param_type: String,
    // TODO
    // /// Parameter value
    // pub value: String,
    // pub value_decoded: Vec<String>
}

/// Decoded function call
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedData {
    /// Method Name
    pub method: String,
    /// Call arguments
    pub parameters: Vec<Parameter>,
}

/// Confirmation info for a multisig transaction
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsigConfirmationResponse {
    /// Which owner this confirmation was produced by
    pub owner: Address,
    /// Date at which the confirmation was submitted
    pub submission_date: String,
    /// TODO: what is this?
    pub transaction_hash: Option<H256>,
    /// The signatures string, in RSV format
    pub signature: String,
    /// The signature type
    /// TODO: Should this be an enum? With what variants
    pub signature_type: String,
}

/// A Multisig History Transaction
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsigTxResponse {
    /// Address of the safe
    pub safe: Address,
    /// Target of the transaction
    pub to: Address,
    /// Native asset value included in the transaction
    #[serde(default, with = "crate::rpc::common::dec_u256_ser")]
    pub value: U256,
    /// Data payload sent to target by safe
    #[serde(default)]
    pub data: Option<Bytes>,
    /// CALL or DELEGATECALL
    pub operation: Operations,
    /// token used to refund gas, address(0) for native asset
    pub gas_token: Address,
    /// Refundable gas that can be used by the safe for sig checking & admin
    pub safe_tx_gas: u64,
    /// TODO: What is this?
    pub base_gas: u64,
    /// The gas price at which to refund the executor (0 if no refund)
    #[serde(with = "crate::rpc::common::dec_u256_ser")]
    pub gas_price: U256,
    /// Address to which to issue gas refunds
    #[serde(deserialize_with = "crate::rpc::common::deser_addr_permit_null")]
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
    /// ETH gas price in the executing transaction. None if unexecuted
    #[serde(default, with = "crate::rpc::common::dec_u256_opt_ser")]
    pub eth_gas_price: Option<U256>,
    /// Max fee per gas in the executing transaction. None if unexecuted
    #[serde(default, with = "crate::rpc::common::dec_u256_opt_ser")]
    pub max_fee_per_gas: Option<U256>,
    /// Max priority fee per gas in the executing transaction. None if
    /// unexecuted
    #[serde(default, with = "crate::rpc::common::dec_u256_opt_ser")]
    pub max_priority_fee_per_gas: Option<U256>,
    /// Gas used in the executing transaction. None if unexecuted
    #[serde(default)]
    pub gas_used: Option<u32>,
    /// Fee used in the executing transaction. None if unexecuted
    #[serde(default, with = "crate::rpc::common::dec_u256_opt_ser")]
    pub fee: Option<U256>,
    /// TODO: what is this?
    #[serde(default)]
    pub origin: Option<String>, // is this correct?
    /// Decoded data (if any)
    /// TODO: what is this?
    #[serde(default)]
    pub data_decoded: Option<DecodedData>,
    /// Confirmations required for the transaction, if any
    #[serde(default)]
    pub confirmations_required: Option<u32>,
    /// Confirmations of the transaction by owners
    pub confirmations: Vec<MsigConfirmationResponse>,
    /// TODO: what is this?
    pub trusted: bool,
    /// TODO: what is this?
    pub signatures: Option<String>, // RSV strings, tightly packed
}

/// Msig History Request
#[derive(serde::Serialize, Clone)]
pub struct MsigHistoryFilters<'a> {
    #[serde(flatten)]
    pub(crate) filters: HashMap<&'static str, String>,
    #[serde(skip)]
    pub(crate) client: &'a SafeClient,
}

impl<'a> AsRef<HashMap<&'static str, String>> for MsigHistoryFilters<'a> {
    fn as_ref(&self) -> &HashMap<&'static str, String> {
        &self.filters
    }
}

impl<'a> MsigHistoryFilters<'a> {
    // TODO: `modified` filters
    // TODO: Execution date & submission date

    // deliberately not supporting LT and GT. redundant
    const NONCE_KEYS: &'static [&'static str] = &["nonce__gte", "nonce__lte", "nonce"];

    // GT and GTE not supported by API for some reason
    const VALUE_KEYS: &'static [&'static str] = &["value__gt", "value__lte", "value"];

    /// Dispatch the request to the API, querying txns from the specified safe
    pub async fn query(self, safe_address: Address) -> ClientResult<MsigHistoryResponse> {
        self.client.filtered_msig_history(safe_address, &self).await
    }

    /// Insert a KV pair into the internal mapping for later URL encoding
    ///
    /// Somewhat more expensive and brittle than required, as it uses
    /// serde_json. Using display would cause hashes and addresses to be
    /// abbreviated `0xabcd....1234`
    fn insert<S: Serialize>(&mut self, k: &'static str, v: S) {
        self.filters.insert(k, serde_json::to_string(&v).unwrap());
    }

    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url, safe_address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/multisig-transactions/",
            ethers::utils::to_checksum(&safe_address, None)
        );
        let mut url = root.clone();
        url.set_path(&path);
        url
    }

    /// Instantiate from a client
    pub(crate) fn new(client: &'a SafeClient) -> Self {
        Self {
            filters: Default::default(),
            client,
        }
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

    /// Filter txns with `nonce >= min_nonce`
    /// Clearns any exact nonce filter
    pub fn min_nonce(mut self, min_nonce: u32) -> Self {
        self.filters.remove("nonce");
        self.insert("nonce__gte", min_nonce);
        self
    }

    /// Filter txns with `nonce <= max_nonce`
    /// Clearns any exact nonce filter
    pub fn max_nonce(mut self, max_nonce: u32) -> Self {
        self.filters.remove("nonce");
        self.insert("nonce__lte", max_nonce);
        self
    }

    /// Filter by exact nonce
    /// Clears any min or max nonce filter
    pub fn nonce(mut self, nonce: u32) -> Self {
        self.clear_nonces();
        self.insert("nonce", nonce);
        self
    }

    /// Filter by exact safe tx hash
    pub fn safe_tx_hash(mut self, h: impl Into<H256>) -> Self {
        self.insert::<H256>("safe_tx_hash", h.into());
        self
    }

    /// Filter by target
    pub fn to(mut self, addr: Address) -> Self {
        self.insert("safe_tx_hash", addr);
        self
    }

    /// Filter txns with `value <= min_value`
    /// Clearns any exact value filter
    pub fn min_value(mut self, value: u64) -> Self {
        self.filters.remove("value");
        self.insert("value__gt", value.saturating_sub(1));
        self
    }

    /// Filter txns with `value <= max_value`
    /// Clearns any exact value filter
    pub fn max_value(mut self, value: u64) -> Self {
        self.filters.remove("value");
        self.insert("value__lt", value.saturating_add(1));
        self
    }

    /// Filter by exact value
    /// Clears any min or max value filter
    pub fn value(mut self, value: u64) -> Self {
        self.clear_values();
        self.insert("value", value);
        self
    }

    /// Filter by execution status
    ///
    /// TODO: what are the acceptable values here? Should this be an enum?
    pub fn executed(mut self, executed: bool) -> Self {
        self.insert("executed", executed.to_string());
        self
    }

    /// Filter by trusted status
    ///
    /// TODO: what are the acceptable values here? Should this be an enum?
    pub fn trusted(mut self, trusted: bool) -> Self {
        self.insert("trusted", trusted.to_string());
        self
    }

    /// Filter by execution transaction hash
    pub fn transaction_hash(mut self, transaction_hash: impl Into<H256>) -> Self {
        self.insert("transaction_hash", transaction_hash.into());
        self
    }

    /// Specify results ordering
    ///
    /// TODO: what are the acceptable values here? Should this be an enum?
    pub fn ordering(mut self, ordering: &str) -> Self {
        self.insert("ordering", ordering.to_owned());
        self
    }

    /// Specify page limit. If more results than limit are returned, results in
    /// a paginated response
    pub fn limit(mut self, limit: u32) -> Self {
        self.insert("limit", limit);
        self
    }

    /// Specify offset in results. Used in pagination, not recommended to be
    /// specified manually
    pub fn offset(mut self, offset: u32) -> Self {
        self.insert("offset", offset);
        self
    }

    /// Converts to a URL with query string
    pub fn to_url(self, safe_address: Address) -> Url {
        let mut url = self.client.url().clone();
        url = Self::url(&url, safe_address);
        url.query_pairs_mut().extend_pairs(self.filters.iter());
        url
    }

    /// Convert to a stream of msig history entries, traversing pages if
    /// necessary
    pub fn into_stream(
        self,
        safe_address: Address,
    ) -> impl tokio_stream::Stream<Item = ClientResult<MsigTxResponse>> + 'a {
        stream! {
            tracing::debug!(
                safe_address = ?safe_address,
                "streaming msig history",
            );
            let Paginated::<MsigTxResponse> {
                mut next,
                results,
                ..
            } = self.query(safe_address).await?;

            for result in results.into_iter() {
                yield Ok(result)
            }
            while let Some(url) = next.take() {
                tracing::debug!(
                    safe_address = ?safe_address,
                    url = %url,
                    "successive page of msig history",
                );
                // Todo: fix to API response
                let Paginated::<MsigTxResponse> {
                    next: n, // avoid shadowing
                    results, // don't care if shadowing
                    ..
                } = serde_json::from_str(&reqwest::get(url).await?.text().await?)?;

                for result in results.into_iter() {
                    yield Ok(result)
                }
                next = n;
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::MsigTxResponse;

    #[test]
    fn it_parses() {
        #[derive(serde::Deserialize)]
        struct Shape {
            results: Vec<serde_json::Value>,
        }

        let f = std::fs::read_to_string("/Users/james/devel/safe/safe-sdk/tmp.json").unwrap();

        let s: Shape = serde_json::from_str(&f).unwrap();
        for (i, result) in s.results.into_iter().enumerate() {
            if let Err(e) = serde_json::from_value::<MsigTxResponse>(result) {
                dbg!(e);
                dbg!(i);
            }
        }
    }
}
