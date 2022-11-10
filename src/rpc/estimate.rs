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
    pub fn url(root: &Url, address: Address) -> reqwest::Url {
        let path = format!("v1/safes/{:?}/multisig-transactions/estimations/", address);
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct EstimateResponse {
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
