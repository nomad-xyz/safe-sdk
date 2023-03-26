use std::collections::HashMap;

use ethers::types::Address;
use reqwest::Url;
use serde::Serialize;

use crate::{client::ClientResult, SafeClient};

use super::common::Paginated;

/// token info request
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenInfoRequest;

impl TokenInfoRequest {
    /// Return the URL to which to dispatch this request
    pub fn url(root: &Url) -> Url {
        let mut url = root.clone();
        url.set_path("api/v1/tokens/");
        url
    }
}

/// Token info Request with filters
#[derive(Clone, Serialize)]
pub struct TokenInfoFilters<'a> {
    #[serde(flatten)]
    pub(crate) filters: HashMap<&'static str, String>,
    #[serde(skip)]
    pub(crate) client: &'a SafeClient,
}

impl<'a> AsRef<HashMap<&'static str, String>> for TokenInfoFilters<'a> {
    fn as_ref(&self) -> &HashMap<&'static str, String> {
        &self.filters
    }
}

impl<'a> TokenInfoFilters<'a> {
    const DECIMAL_KEYS: &'static [&'static str] = &["decimals__lt", "decimals__gt", "decimals"];

    /// Dispatch the request to the API, querying tokens from the API
    pub async fn query(self) -> ClientResult<TokenInfoResponse> {
        self.client.filtered_tokens(&self).await
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
    pub fn url(root: &Url) -> Url {
        let mut url = root.clone();
        url.set_path("api/v1/tokens/");
        url
    }

    /// Instantiate from a client
    pub(crate) fn new(client: &'a SafeClient) -> Self {
        Self {
            filters: Default::default(),
            client,
        }
    }

    fn clear_decimals(&mut self) {
        for k in Self::DECIMAL_KEYS {
            self.filters.remove(k);
        }
    }

    /// Filters tokens by name
    pub fn name(mut self, name: String) -> Self {
        self.filters.insert("name", name);
        self
    }

    /// Filter tokens by address
    pub fn address(mut self, address: Address) -> Self {
        self.insert("address", address);
        self
    }

    /// Filter tokens by symbol
    pub fn symbol(mut self, symbol: String) -> Self {
        self.filters.insert("symbol", symbol);
        self
    }

    /// Filter tokens with `decimals >= min_decimals`
    /// Clears any exact decimals filter
    pub fn min_decimals(mut self, decimals: u64) -> Self {
        self.filters.remove("decimals");
        self.insert("decimals__gt", decimals.saturating_sub(1));
        self
    }

    /// Filter tokens with `decimals <= max_decimals`
    /// Clears any exact decimals filter
    pub fn max_decimals(mut self, decimals: u64) -> Self {
        self.filters.remove("decimals");
        self.insert("decimals__lt", decimals.saturating_add(1));
        self
    }

    /// Filter tokens by exact decimals
    /// Clears any min or max decimals filters
    pub fn decimals(mut self, decimals: u64) -> Self {
        self.clear_decimals();
        self.insert("decimals", decimals);
        self
    }

    /// Specify page limit. If more results than limit are returned, results in
    /// a paginated response
    pub fn limit(mut self, limit: u64) -> Self {
        self.insert("limit", limit);
        self
    }

    /// Specify offset in results. Used in pagination, not recommended to be
    /// specified manually
    pub fn offset(mut self, offset: u64) -> Self {
        self.insert("offset", offset);
        self
    }

    /// Converts to a URL with query string
    pub fn to_url(self) -> Url {
        let mut url = self.client.url().clone();
        url = Self::url(&url);
        url.query_pairs_mut().extend_pairs(self.filters.iter());
        url
    }
}

/// The type of the token (ERC20, ERC721, etc)
#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize)]
pub enum TokenType {
    /// ERC20 type
    ERC20,
    /// ERC721 type
    ERC721,
    /// ERC1155 type
    ERC1155,
}

impl From<String> for TokenType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "ERC20" => TokenType::ERC20,
            "ERC721" => TokenType::ERC721,
            "ERC1155" => TokenType::ERC1155,
            _ => panic!("Unknown token type"),
        }
    }
}

/// token info response
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenIfResponse {
    /// The token type (ERC20, ERC721, etc)
    #[serde(rename(deserialize = "type"))]
    // #[serde(skip)]
    pub token_type: TokenType,
    /// The address of the token
    pub address: String,
    /// The name of the token
    pub name: String,
    /// The symbol of the token
    pub symbol: String,
    /// The number of decimals of the token
    pub decimals: Option<u32>,
    /// The Logo URI of the token, if it exists
    #[serde(rename(deserialize = "logoUri"))]
    // #[serde(skip)]
    pub logo_uri: String,
}

/// Response for Token Info requests
pub type TokenInfoResponse = Paginated<TokenIfResponse>;
