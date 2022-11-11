use ethers::{
    providers::{FromErr, Middleware},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, Signature},
};
use tokio::{
    sync::{RwLock, RwLockReadGuard},
    try_join,
};

use crate::{
    client::{SigningClient, SigningClientError},
    rpc::{
        common::Operations,
        propose::{MetaTransactionData, ProposeRequest, SafeGasConfig, SafeTransactionData},
    },
    ClientError,
};

#[derive(thiserror::Error, Debug)]
pub enum SafeMiddlewareError<M, S>
where
    M: Middleware,
    S: Signer,
{
    /// Thrown when the internal middleware errors
    #[error("{0}")]
    MiddlewareError(M::Error),
    /// Signing Client Error
    #[error("{0}")]
    SigningClientError(#[from] SigningClientError<S>),
    /// Incomplete tx details, does not specify to
    #[error("Transaction must specify to address")]
    MissingTo,
}

impl<M, S> From<ClientError> for SafeMiddlewareError<M, S>
where
    M: Middleware,
    S: Signer,
{
    fn from(e: ClientError) -> Self {
        SafeMiddlewareError::SigningClientError(e.into())
    }
}

impl<M, S> FromErr<M::Error> for SafeMiddlewareError<M, S>
where
    M: Middleware,
    S: Signer,
{
    fn from(src: M::Error) -> Self {
        SafeMiddlewareError::MiddlewareError(src)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SafeMiddlewareConfig {
    pub submit_to_service: bool,
    pub default_operation: Operations,
    pub gas: SafeGasConfig,
}

impl Default for SafeMiddlewareConfig {
    fn default() -> Self {
        Self {
            submit_to_service: true,
            default_operation: Operations::Call,
            gas: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct SafeMiddleware<M, S> {
    safe_address: Address,
    inner: M,
    client: SigningClient<S>,
    config: SafeMiddlewareConfig,
    proposals: RwLock<Vec<ProposeRequest>>,
}

impl<M, S> SafeMiddleware<M, S> {
    pub async fn proposals(&self) -> RwLockReadGuard<Vec<ProposeRequest>> {
        self.proposals.read().await
    }
}

impl<M, S> SafeMiddleware<M, S>
where
    M: Middleware,
    S: Signer + 'static,
{
    /// Overrides signer chain_id with provider's
    pub async fn try_from_signer(
        safe_address: Address,
        inner: M,
        signer: S,
    ) -> Result<Self, SafeMiddlewareError<M, S>> {
        let chain_id = inner
            .get_chainid()
            .await
            .map_err(SafeMiddlewareError::<M, S>::MiddlewareError)?;

        let signer = signer.with_chain_id(chain_id.low_u64());

        let client = SigningClient::try_from_signer(signer)?;

        Ok(Self {
            safe_address,
            inner,
            client,
            config: Default::default(),
            proposals: RwLock::new(Default::default()),
        })
    }

    async fn to_meta_tx<'a>(
        &self,
        tx: &'a TypedTransaction,
    ) -> Result<MetaTransactionData, SafeMiddlewareError<M, S>> {
        let to = tx.to().ok_or(SafeMiddlewareError::MissingTo)?;
        let to = match to {
            ethers::types::NameOrAddress::Name(name) => self
                .inner()
                .resolve_name(name)
                .await
                .map_err(SafeMiddlewareError::MiddlewareError)?,
            ethers::types::NameOrAddress::Address(addr) => *addr,
        }
        .into();

        let value = tx.value().copied().unwrap_or_default();
        let data = tx.data().cloned();

        Ok(MetaTransactionData {
            to,
            value: value.low_u64(),
            data,
            operation: None,
        })
    }
}

#[async_trait::async_trait]
impl<M, S> Middleware for SafeMiddleware<M, S>
where
    S: Signer + 'static,
    M: Middleware,
{
    type Error = SafeMiddlewareError<M, S>;

    type Provider = M::Provider;

    type Inner = M;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    /// Sign a transaction via RPC call
    async fn sign_transaction(
        &self,
        tx: &TypedTransaction,
        _from: Address,
    ) -> Result<Signature, Self::Error> {
        // in order to use shortcutting try_join, we have to have all error
        // types be the same. So 1 future needs to be wrapped & mapped

        let (mut core, chain_id, info) =
            try_join!(self.to_meta_tx(tx), self.get_chainid(), async {
                Ok(self.client.safe_info(self.safe_address).await?)
            },)?;

        // TODO: user configurable
        // but tbh the UI just sets these to 0 so........
        let SafeMiddlewareConfig {
            submit_to_service,
            default_operation,
            gas,
        } = self.config;

        // override from config if necessary
        if core.operation.is_none() {
            core.operation = Some(default_operation)
        }

        let proposal = SafeTransactionData {
            core,
            gas,
            nonce: info.nonce,
        };

        let proposal = proposal
            .into_request(&self.client.signer, self.safe_address, chain_id.low_u64())
            .await
            .map_err(SigningClientError::<S>::SignerError)?;

        // guard dropped immediately on use
        self.proposals.write().await.push(proposal.clone());

        let signature = proposal.signature.signature();
        if submit_to_service {
            self.client
                .submit_proposal(proposal, self.safe_address)
                .await?;
        }
        Ok(signature)
    }
}
