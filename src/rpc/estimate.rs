use ethers::types::{Address, Bytes, U256};
use reqwest::Url;

use super::common::Operations;

#[derive(Debug, serde::Serialize)]
/// Estimates `safe_tx_gas` for a proposed msig txn
pub struct EstimateRequest<'a> {
    pub(crate) to: Address,
    pub(crate) value: u64,
    #[serde(serialize_with = "crate::rpc::common::default_empty_bytes_ref")]
    pub(crate) data: Option<&'a Bytes>,
    pub(crate) operation: Operations,
}

impl<'a> EstimateRequest<'a> {
    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url, address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/multisig-transactions/estimations/",
            ethers::utils::to_checksum(&address, None)
        );
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

/// Response of the estimate endpoint
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct EstimateResponse {
    /// The amount of gas estimated
    pub safe_tx_gas: U256,
}

impl std::ops::Deref for EstimateResponse {
    type Target = U256;

    fn deref(&self) -> &Self::Target {
        &self.safe_tx_gas
    }
}

impl From<EstimateResponse> for U256 {
    fn from(resp: EstimateResponse) -> Self {
        resp.safe_tx_gas
    }
}
