use ethers::types::{Address, U256};
use reqwest::Url;

pub struct SafeInfoRequest;

impl SafeInfoRequest {
    pub fn url(root: &Url, address: Address) -> reqwest::Url {
        let path = format!("v1/safes/{:?}/", address);
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeInfoResponse {
    address: Address,
    nonce: U256,
    threshold: u32,
    owners: Vec<Address>,
    master_copy: Address,
    modules: Vec<String>,
    fallback_handler: Address,
    guard: Address,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}
