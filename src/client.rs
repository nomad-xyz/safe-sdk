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
        self,
        estimate::{EstimateRequest, EstimateResponse},
        info::{SafeInfoRequest, SafeInfoResponse},
        msig_history::{
            MsigHistoryRequest, MsigHistoryResponse, SafeMultiSigTxRequest,
            SafeMultisigTransactionResponse,
        },
        propose::{MetaTransactionData, ProposeRequest, SafeTransactionData},
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
    /// server status other than 422
    #[error("Server Error {0}")]
    ServerErrorCode(StatusCode),
    /// API Error
    #[error("API usage error: {0}")]
    ApiError(rpc::common::ErrorResponse),
    /// No known service endpoint for chain_id
    #[error("No known service URL for chain id {0}. Hint: if using a custom tx service api, specify via a `TxService` object, rather than via a chain id.")]
    UnknownServiceId(u64),
    /// Other Error
    #[error("{0}")]
    Other(String),
}

impl From<rpc::common::ErrorResponse> for ClientError {
    fn from(err: rpc::common::ErrorResponse) -> Self {
        Self::ApiError(err)
    }
}

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
pub type ClientResult<T> = Result<T, ClientError>;
pub type SigningClientResult<T, S> = Result<T, SigningClientError<S>>;

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
    pub fn by_chain_id(chain_id: u64) -> Option<Self> {
        TxService::by_chain_id(chain_id).map(Into::into)
    }

    pub fn ethereum() -> Self {
        networks::ETHEREUM.into()
    }

    pub fn new(network: TxService) -> Self {
        network.into()
    }

    pub fn with_client(network: TxService, client: reqwest::Client) -> Self {
        Self {
            service: network,
            client,
            url_cache: Url::parse(network.url).unwrap(),
        }
    }
    /// network URL
    pub fn url(&self) -> &Url {
        &self.url_cache
    }

    pub fn with_signer<S: Signer>(self, signer: S) -> SigningClient<S> {
        SigningClient {
            client: self,
            signer,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn safe_info(&self, address: Address) -> ClientResult<SafeInfoResponse> {
        json_get!(
            &self.client,
            SafeInfoRequest::url(self.url(), address),
            SafeInfoResponse,
        )
        .map(Option::unwrap)
    }

    pub async fn msig_history(&self, address: Address) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryRequest::url(self.url(), address),
            MsigHistoryResponse
        )
        .map(Option::unwrap)
    }

    /// Get the highest unused nonce
    pub async fn next_nonce(&self, address: Address) -> ClientResult<u64> {
        Ok(self.msig_history(address).await?.count)
    }

    pub(crate) async fn filtered_msig_history(
        &self,
        address: Address,
        filters: &HashMap<&'static str, String>,
    ) -> ClientResult<MsigHistoryResponse> {
        json_get!(
            &self.client,
            MsigHistoryRequest::url(self.url(), address),
            MsigHistoryResponse,
            filters,
        )
        .map(Option::unwrap)
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
            EstimateRequest::<'a>::url(self.url(), address),
            &req
        )
        .map(|resp: Option<EstimateResponse>| resp.unwrap().into())
    }

    pub fn network(&self) -> TxService {
        self.service
    }

    pub async fn transaction_info(
        &self,
        tx_hash: H256,
    ) -> ClientResult<SafeMultisigTransactionResponse> {
        json_get!(
            &self.client,
            SafeMultiSigTxRequest::url(self.url(), tx_hash),
            SafeMultisigTransactionResponse
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
    pub fn try_from_signer(signer: S) -> Result<Self, ClientError> {
        let chain_id = signer.chain_id();
        match TxService::by_chain_id(chain_id) {
            Some(service) => Ok(Self::with_service_and_signer(service, signer)),
            None => Err(ClientError::UnknownServiceId(chain_id)),
        }
    }

    pub fn ethereum(signer: S) -> Self {
        Self::with_service_and_signer(networks::ETHEREUM, signer)
    }

    pub fn with_service_and_signer(service: TxService, signer: S) -> Self {
        let signer = signer.with_chain_id(service.chain_id);
        SafeClient::from(service).with_signer(signer)
    }

    pub async fn submit_proposal(
        &self,
        proposal: ProposeRequest,
        safe_address: Address,
    ) -> SigningClientResult<SafeMultisigTransactionResponse, S> {
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

    pub async fn propose_tx(
        &self,
        tx: SafeTransactionData,
        safe_address: Address,
    ) -> SigningClientResult<SafeMultisigTransactionResponse, S> {
        let proposal = tx
            .into_request(&self.signer, safe_address, self.signer.chain_id())
            .await
            .map_err(SigningClientError::SignerError)?;
        self.submit_proposal(proposal, safe_address).await
    }

    pub async fn propose(
        &self,
        tx: impl Into<MetaTransactionData>,
        safe_address: Address,
    ) -> SigningClientResult<SafeMultisigTransactionResponse, S> {
        let nonce = self.next_nonce(safe_address).await?;
        tracing::info!(nonce);
        let proposal = SafeTransactionData {
            core: tx.into(),
            gas: Default::default(),
            nonce,
        };
        self.propose_tx(proposal, safe_address).await
    }
}
