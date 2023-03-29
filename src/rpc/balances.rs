use std::collections::HashMap;

use ethers::types::{Address, U256};
use reqwest::Url;

use crate::{client::ClientResult, SafeClient};

/// Safe balances response
pub type BalancesResponse = Vec<BalanceResponse>;

/// Safe balances request
#[derive(Debug, Clone, serde::Serialize)]
pub struct BalancesRequest;

impl BalancesRequest {
    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url, safe_address: Address) -> Url {
        let mut url = root.clone();
        url.set_path(&format!(
            "api/v1/safes/{}/balances/usd/",
            ethers::utils::to_checksum(&safe_address, None)
        ));
        url
    }
}

/// Safe Balances request filters
#[derive(Clone, serde::Serialize)]
pub struct BalancesFilters<'a> {
    #[serde(flatten)]
    pub(crate) filters: HashMap<&'static str, String>,
    #[serde(skip)]
    pub(crate) client: &'a SafeClient,
}

impl<'a> BalancesFilters<'a> {
    /// Dispatch the request to the API, querying safe balances from the API
    pub async fn query(self, safe_address: Address) -> ClientResult<BalancesResponse> {
        self.client
            .filtered_balances(safe_address, self.filters)
            .await
    }

    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url, safe_address: Address) -> reqwest::Url {
        let path = format!(
            "api/v1/safes/{}/balances/usd/",
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

    /// Filter by allowing only trusted tokens to be returned or not.
    pub fn trusted(mut self, trusted: bool) -> Self {
        self.filters.insert("trusted", trusted.to_string());
        self
    }

    /// Filter by allowing known spam tokens to be returned or not.
    pub fn exclude_spam(mut self, exclude_spam: bool) -> Self {
        self.filters
            .insert("exclude_spam", exclude_spam.to_string());
        self
    }
}

/// The individual response for every Safe token balance.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    /// The address of the token (null for native tokens)
    pub token_address: Option<Address>,
    /// The token info (null for native tokens)
    pub token: Option<Erc20Info>,
    /// The balance of the safe for the token
    pub balance: U256,
    /// The value in eth of the token
    pub eth_value: String,
    /// The timestamp of when the conversion was made
    pub timestamp: String,
    /// The balance in USD of the token
    pub fiat_balance: String,
    /// The conversion rate used to calculate the fiat balance
    pub fiat_conversion: String,
    /// The currency used to calculate the fiat balance
    pub fiat_code: String,
}

/// The info about the token, if it's an ERC20 token.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Erc20Info {
    /// The name of the token
    pub name: String,
    /// The token symbol
    pub symbol: String,
    /// The token decimals
    pub decimals: Option<u32>,
    /// The logo URI, if it exists.
    pub logo_uri: Option<String>,
}
