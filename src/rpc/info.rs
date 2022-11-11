use ethers::types::Address;
use reqwest::Url;

pub struct SafeInfoRequest;

impl SafeInfoRequest {
    pub fn url(root: &Url, address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/",
            ethers::utils::to_checksum(&address, None)
        );
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeInfoResponse {
    pub address: Address,
    pub nonce: u64,
    pub threshold: u32,
    pub owners: Vec<Address>,
    pub master_copy: Address,
    pub modules: Vec<String>,
    pub fallback_handler: Address,
    pub guard: Address,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
