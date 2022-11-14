use ethers::types::Address;
use reqwest::Url;

/// Safe info request (no params needed)
pub struct SafeInfoRequest;

impl SafeInfoRequest {
    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url, safe_address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/",
            ethers::utils::to_checksum(&safe_address, None)
        );
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}

/// SAFE Info tracked by the API
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeInfoResponse {
    /// The Safe's address
    #[serde(rename = "address")]
    pub safe_address: Address,
    /// The current on-chain nonce (not counting any pending txns)
    pub nonce: u64,
    /// The number of required signers
    pub threshold: u32,
    /// A list of the Owners
    pub owners: Vec<Address>,
    /// The implementation address this safe proxies
    pub master_copy: Address,
    /// Any modules this safe uses
    pub modules: Vec<String>,
    /// The fallback handler for this safe (0 if none)
    pub fallback_handler: Address,
    /// The guard for this safe (0 if none)
    pub guard: Address,
    /// The safe version string
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
