use ethers::{
    providers::{FromErr, Middleware},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, Signature, U256},
};
use tokio::{
    join,
    sync::{RwLock, RwLockReadGuard},
};

use crate::{
    client::{SigningClient, SigningClientError},
    rpc::propose::{MetaTransactionData, ProposeRequest, SafeTransactionData},
    ClientError,
};

#[derive(thiserror::Error, Debug)]
pub enum SafeMiddlewareError<S, M>
where
    S: Signer,
    M: Middleware,
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

impl<S, M> From<ClientError> for SafeMiddlewareError<S, M>
where
    S: Signer,
    M: Middleware,
{
    fn from(e: ClientError) -> Self {
        SafeMiddlewareError::SigningClientError(e.into())
    }
}

impl<S, M> FromErr<M::Error> for SafeMiddlewareError<S, M>
where
    S: Signer,
    M: Middleware,
{
    fn from(src: M::Error) -> Self {
        SafeMiddlewareError::MiddlewareError(src)
    }
}

#[derive(Debug)]
pub struct SafeMiddleware<M, S> {
    inner: M,
    client: SigningClient<S>,
    pub submit_to_api: bool,
    proposals: RwLock<Vec<ProposeRequest>>,
    safe_address: Address,
}

impl<M, S> SafeMiddleware<M, S> {
    pub fn new(
        inner: M,
        client: SigningClient<S>,
        submit_to_api: bool,
        safe_address: Address,
    ) -> Self {
        Self {
            inner,
            client,
            submit_to_api,
            proposals: RwLock::new(Vec::new()),
            safe_address,
        }
    }

    pub async fn proposals(&self) -> RwLockReadGuard<Vec<ProposeRequest>> {
        self.proposals.read().await
    }
}

impl<S, M> SafeMiddleware<M, S>
where
    S: Signer + 'static,
    M: Middleware,
{
    async fn to_meta_tx<'a>(
        &self,
        tx: &'a TypedTransaction,
    ) -> Result<MetaTransactionData, SafeMiddlewareError<S, M>> {
        let to = tx.to().ok_or(SafeMiddlewareError::MissingTo)?;
        let to = match to {
            ethers::types::NameOrAddress::Name(name) => self
                .inner()
                .resolve_name(name)
                .await
                .map_err(SafeMiddlewareError::MiddlewareError)?, // TODO
            ethers::types::NameOrAddress::Address(addr) => *addr,
        };

        let value = tx.value().copied().unwrap_or_default();
        let data = tx.data().cloned();

        Ok(MetaTransactionData {
            to,
            value,
            data,
            operation: None,
        })
    }
}

#[async_trait::async_trait]
impl<S, M> Middleware for SafeMiddleware<M, S>
where
    S: Signer + 'static,
    M: Middleware,
{
    type Error = SafeMiddlewareError<S, M>;

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
        let (core, chain_id) = join!(self.to_meta_tx(tx), self.get_chainid());
        let (core, chain_id) = (core?, chain_id?);

        // TODO: user config
        let safe_tx_gas = U256::zero();
        let base_gas = U256::zero();
        let gas_price = U256::zero();
        let gas_token = Address::default();
        let refund_receiver = Address::zero();
        let nonce = self.client.safe_info(self.safe_address).await?.nonce;

        let proposal = SafeTransactionData {
            core,
            safe_tx_gas,
            base_gas,
            gas_price,
            gas_token,
            refund_receiver,
            nonce,
        };
        let proposal = proposal
            .into_request(&self.client.signer, self.safe_address, chain_id)
            .await
            .map_err(SigningClientError::<S>::SignerError)?;

        {
            self.proposals.write().await.push(proposal.clone());
        }

        let signature = proposal.signature.signature();
        if self.submit_to_api {
            self.client
                .submit_proposal(proposal, self.safe_address)
                .await?;
        }
        Ok(signature)
    }
}
