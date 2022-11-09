use std::{collections::HashMap, ops::Deref};

use ethers::{
    signers::Signer,
    types::{Address, U256},
};
use reqwest::IntoUrl;

use crate::{
    json_get, json_post,
    rpc::{
        self,
        estimate::{EstimateRequest, EstimateResponse},
        info::{SafeInfoRequest, SafeInfoResponse},
        msig_history::{MsigHistoryRequest, MsigHistoryResponse},
        propose::SafeTransactionData,
    },
};

/// Gnosis Client Errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Reqwest Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    /// Url Parsing Error
    #[error("{0}")]
    UrlParse(#[from] url::ParseError),
    /// Serde Json deser Error
    #[error("{0}")]
    SerdeError(#[from] serde_json::Error),
    /// No Signer
    #[error("Operation requires signer")]
    NoSigner,
    /// Wrong Signer
    #[error("Wrong Signer: Request specified {specified:?}. Client has: {available:?}")]
    WrongSigner {
        specified: Address,
        available: Address,
    },
    /// API Error
    #[error("API usage error: {0}")]
    ApiError(rpc::common::ErrorResponse),
    /// Other Error
    #[error("{0}")]
    Other(String),
}

impl From<rpc::common::ErrorResponse> for ClientError {
    fn from(err: rpc::common::ErrorResponse) -> Self {
        Self::ApiError(err)
    }
}

/// A Safe Transaction Service client
pub struct GnosisClient {
    pub(crate) url: reqwest::Url,
    pub(crate) client: reqwest::Client,
}

/// A Safe Transaction Service client with signing and tx submission
/// capabilities
pub struct SigningClient<S> {
    pub(crate) client: GnosisClient,
    pub(crate) signer: S,
}

impl<S> Deref for SigningClient<S> {
    type Target = GnosisClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// Gnosis Client Results
pub type ClientResult<T> = Result<T, ClientError>;

impl GnosisClient {
    /// Instantiate a new client with a specific URL
    ///
    /// # Errors
    ///
    /// If the url param cannot be parsed as a URL
    pub fn new<S>(url: S) -> ClientResult<Self>
    where
        S: IntoUrl,
    {
        Ok(Self {
            url: url.into_url()?,
            client: Default::default(),
        })
    }

    pub fn with_signer<S: Signer>(self, signer: S) -> SigningClient<S> {
        SigningClient {
            client: self,
            signer,
        }
    }

    /// Instantiate a new client with a specific URL and a reqwest Client
    ///
    /// # Errors
    ///
    /// If the url param cannot be parsed as a URL
    pub fn new_with_client<S>(url: S, client: reqwest::Client) -> ClientResult<Self>
    where
        S: IntoUrl,
    {
        Ok(Self {
            url: url.into_url()?,
            client,
        })
    }

    pub async fn safe_info(&self, address: Address) -> ClientResult<SafeInfoResponse> {
        json_get!(
            &self.client,
            SafeInfoRequest::url(&self.url, address),
            SafeInfoResponse,
        )
    }

    pub async fn msig_history(&self, address: Address) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryRequest::url(&self.url, address),
            MsigHistoryResponse
        )
    }

    /// Get the highest unused nonce
    pub async fn next_nonce(&self, address: Address) -> ClientResult<u32> {
        Ok(self
            .msig_history(address)
            .await?
            .results
            .iter()
            .map(|tx| tx.nonce)
            .max()
            .unwrap_or(0))
    }

    pub(crate) async fn filtered_msig_history(
        &self,
        address: Address,
        filters: &HashMap<&'static str, String>,
    ) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryRequest::url(&self.url, address),
            MsigHistoryResponse,
            filters,
        )
    }

    pub async fn msig_history_builder(&self) -> MsigHistoryRequest {
        MsigHistoryRequest::new(self)
    }

    pub async fn estimate_gas<'a>(
        &self,
        address: Address,
        tx: impl Into<EstimateRequest<'a>>,
    ) -> ClientResult<U256> {
        let req = tx.into();
        json_post!(
            self.client,
            EstimateRequest::<'a>::url(&self.url, address),
            &req
        )
        .map(|resp: EstimateResponse| resp.into())
    }
}

impl<S: Signer> SigningClient<S> {
    pub async fn propose_tx(&self, tx: &SafeTransactionData) -> ClientResult<()> {
        todo!()
    }
}
