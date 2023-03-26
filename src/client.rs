use std::{collections::HashMap, ops::Deref};

use ethers::{
    signers::Signer,
    types::{Address, H256, U256},
};
use reqwest::{StatusCode, Url};

use crate::{
    json_get, json_post,
    networks::{self, TxService},
    rpc::{
        common::ErrorResponse,
        estimate::{EstimateRequest, EstimateResponse},
        info::{SafeInfoRequest, SafeInfoResponse},
        msig_history::{MsigHistoryFilters, MsigHistoryResponse, MsigTxRequest, MsigTxResponse},
        propose::{MetaTransactionData, ProposeRequest, SafeTransactionData},
        tokens::{TokenInfoFilters, TokenInfoRequest, TokenInfoResponse},
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
        /// Specified signer's address
        specified: Address,
        /// Address of available signer
        available: Address,
    },
    /// server status other than 422
    #[error("Server Error {0}")]
    ServerErrorCode(StatusCode),
    /// API Error
    #[error("API usage error: {0}")]
    ApiError(ErrorResponse),
    /// No known service endpoint for chain_id
    #[error("No known service URL for chain id {0}. Hint: if using a custom tx service api, specify via a `TxService` object, rather than via a chain id.")]
    UnknownServiceId(u64),
    /// Other Error
    #[error("{0}")]
    Other(String),
}

impl From<ErrorResponse> for ClientError {
    fn from(err: ErrorResponse) -> Self {
        Self::ApiError(err)
    }
}

/// Error for SigningClient
#[derive(thiserror::Error, Debug)]
pub enum SigningClientError<S: Signer> {
    /// ClientError
    #[error("{0}")]
    ClientError(ClientError),
    /// SignerError
    #[error("{0}")]
    SignerError(S::Error),
}

impl<T, S> From<T> for SigningClientError<S>
where
    T: Into<ClientError>,
    S: Signer,
{
    fn from(t: T) -> Self {
        Self::ClientError(t.into())
    }
}

/// Gnosis Client Results
pub(crate) type ClientResult<T> = Result<T, ClientError>;

/// Signing Client
pub(crate) type SigningClientResult<T, S> = Result<T, SigningClientError<S>>;

#[derive(Debug)]
/// A Safe Transaction Service client
pub struct SafeClient {
    pub(crate) service: TxService,
    pub(crate) client: reqwest::Client,
    url_cache: Url,
}

impl From<TxService> for SafeClient {
    fn from(network: TxService) -> Self {
        Self {
            service: network,
            client: Default::default(),
            url_cache: Url::parse(network.url).unwrap(),
        }
    }
}

impl Deref for SafeClient {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl SafeClient {
    /// Instantiate a client from a known chain ID by looking up the service URL
    pub fn by_chain_id(chain_id: u64) -> Option<Self> {
        TxService::by_chain_id(chain_id).map(Into::into)
    }

    /// Instantiate a client using the ethereum mainnet service
    pub fn ethereum() -> Self {
        networks::ETHEREUM.into()
    }

    /// Instantiate a client from a Service struct
    pub fn new(network: TxService) -> Self {
        network.into()
    }

    /// Instantiate a client from a Service struct and reqwest client
    pub fn with_client(network: TxService, client: reqwest::Client) -> Self {
        Self {
            service: network,
            client,
            url_cache: Url::parse(network.url).unwrap(),
        }
    }
    /// Return the safe transaction service root URL
    pub fn url(&self) -> &Url {
        &self.url_cache
    }

    /// Return the TxService struct
    pub fn network(&self) -> TxService {
        self.service
    }

    /// Add a signer to the client, to allow proposing transactions
    pub fn with_signer<S: Signer>(self, signer: S) -> SigningClient<S> {
        SigningClient {
            client: self,
            signer,
        }
    }

    /// Get information about the Safe from the API
    #[tracing::instrument(skip(self))]
    pub async fn safe_info(&self, safe_address: Address) -> ClientResult<SafeInfoResponse> {
        json_get!(
            &self.client,
            SafeInfoRequest::url(self.url(), safe_address),
            SafeInfoResponse,
        )
        .map(Option::unwrap)
    }

    /// Get information about tokens available on the API
    #[tracing::instrument(skip(self))]
    pub async fn tokens(&self) -> ClientResult<TokenInfoResponse> {
        json_get!(
            &self.client,
            TokenInfoRequest::url(self.url()),
            TokenInfoResponse,
        )
        .map(Option::unwrap)
    }

    /// Get fitered information about tokens available on the API
    #[tracing::instrument(skip(self, filters))]
    pub async fn filtered_tokens(
        &self,
        filters: impl AsRef<HashMap<&'static str, String>>,
    ) -> ClientResult<TokenInfoResponse> {
        json_get!(
            &self.client,
            TokenInfoRequest::url(self.url()),
            TokenInfoResponse,
            filters.as_ref()
        )
        .map(Option::unwrap)
    }

    /// Create a filter builder for tokens
    #[tracing::instrument(skip(self))]
    pub fn tokens_builder(&self) -> TokenInfoFilters<'_> {
        TokenInfoFilters::new(self)
    }

    /// Get the history of Msig transactions from the API
    #[tracing::instrument(skip(self))]
    pub async fn msig_history(&self, safe_address: Address) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryFilters::url(self.url(), safe_address),
            MsigHistoryResponse
        )
        .map(Option::unwrap)
    }

    /// Get the highest unused nonce, by getting the count of all past txns
    ///
    /// TODO: does this break if the reply is paginated?
    #[tracing::instrument(skip(self))]
    pub async fn next_nonce(&self, safe_address: Address) -> ClientResult<u64> {
        Ok(self.msig_history(safe_address).await?.count)
    }

    /// Request a filtered history of msig txns for the safe
    #[tracing::instrument(skip(self, filters))]
    pub(crate) async fn filtered_msig_history(
        &self,
        safe_address: Address,
        filters: impl AsRef<HashMap<&'static str, String>>,
    ) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryFilters::url(self.url(), safe_address),
            MsigHistoryResponse,
            filters.as_ref(),
        )
        .map(Option::unwrap)
    }

    /// Create a filter builder for msig history
    #[tracing::instrument(skip(self))]
    pub fn msig_history_builder(&self) -> MsigHistoryFilters<'_> {
        MsigHistoryFilters::new(self)
    }

    /// Estimate the safeTxGas to attach to a transaction proposal
    #[tracing::instrument(skip(self, tx))]
    pub async fn estimate_gas<'a>(
        &self,
        safe_address: Address,
        tx: impl Into<EstimateRequest<'a>>,
    ) -> ClientResult<U256> {
        let req = tx.into();
        json_post!(
            self.client,
            EstimateRequest::<'a>::url(self.url(), safe_address),
            &req
        )
        .map(|resp: Option<EstimateResponse>| resp.unwrap().into())
    }

    /// Get the details of a transaction. Errors on unknown transaction
    #[tracing::instrument(skip(self))]
    pub async fn transaction_info(&self, tx_hash: H256) -> ClientResult<MsigTxResponse> {
        json_get!(
            &self.client,
            MsigTxRequest::url(self.url(), tx_hash),
            MsigTxResponse
        )
        .map(Option::unwrap)
    }
}

#[derive(Debug)]
/// A Safe Transaction Service client with signing and tx submission
/// capabilities
pub struct SigningClient<S> {
    pub(crate) client: SafeClient,
    pub(crate) signer: S,
}

impl<S> Deref for SigningClient<S> {
    type Target = SafeClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<S: Signer> SigningClient<S> {
    /// Instantiate a signing client from a signer, by looking up the chain id
    /// in known services
    pub fn try_from_signer(signer: S) -> Result<Self, ClientError> {
        let chain_id = signer.chain_id();
        match TxService::by_chain_id(chain_id) {
            Some(service) => Ok(Self::with_service_and_signer(service, signer)),
            None => Err(ClientError::UnknownServiceId(chain_id)),
        }
    }

    /// Instantiate a signing client from a signer, on ethereum mainnet
    pub fn ethereum(signer: S) -> Self {
        Self::with_service_and_signer(networks::ETHEREUM, signer)
    }

    /// Instantiate a signing client from a service & signer, overriding the
    /// signer's chain ID with the service's chain ID
    pub fn with_service_and_signer(service: TxService, signer: S) -> Self {
        let signer = signer.with_chain_id(service.chain_id);
        SafeClient::from(service).with_signer(signer)
    }

    /// Submit a signed proposal request for storage on the API
    pub async fn submit_proposal(
        &self,
        proposal: ProposeRequest,
        safe_address: Address,
    ) -> SigningClientResult<MsigTxResponse, S> {
        let tx_hash = proposal.safe_tx_hash();
        // little crufty. TODO: fix macro more gooder
        json_post!(
            self.client,
            ProposeRequest::url(self.url(), safe_address),
            &proposal
        )
        .map(|_: Option<()>| ())?;
        Ok(self.transaction_info(tx_hash).await?)
    }

    /// Propose a SafeTransaction to the API. First signs the transaction
    /// request with the signer, then submits
    pub async fn propose_tx(
        &self,
        tx: SafeTransactionData,
        safe_address: Address,
    ) -> SigningClientResult<MsigTxResponse, S> {
        let proposal = tx
            .into_request(&self.signer, safe_address, self.signer.chain_id())
            .await
            .map_err(SigningClientError::SignerError)?;
        self.submit_proposal(proposal, safe_address).await
    }

    /// Propose a transaction to the API. Converts to a Safe Transaction, then
    /// signs, then submits
    ///
    /// TODO: more implementations of `From<X> for MetaTransactionData`
    pub async fn propose(
        &self,
        tx: impl Into<MetaTransactionData>,
        safe_address: Address,
    ) -> SigningClientResult<MsigTxResponse, S> {
        let nonce = self.next_nonce(safe_address).await?;
        let proposal = SafeTransactionData {
            core: tx.into(),
            gas: Default::default(),
            nonce,
        };
        self.propose_tx(proposal, safe_address).await
    }
}
