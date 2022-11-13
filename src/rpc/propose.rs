use std::convert::Infallible;

use ethers::{
    abi::{self, Tokenize},
    signers::Signer,
    types::{
        transaction::eip712::{EIP712Domain, Eip712},
        Address, Bytes, Signature, H256, U256,
    },
    utils::keccak256,
};
use reqwest::Url;

use crate::rpc::common::{Operations, DOMAIN_SEPARATOR_TYPEHASH};

use super::{
    common::{ChecksumAddress, SAFE_TX_TYPEHASH},
    estimate::EstimateRequest,
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetaTransactionData {
    pub to: ChecksumAddress,
    pub value: u64,
    #[serde(serialize_with = "crate::rpc::common::default_empty_bytes")]
    pub data: Option<Bytes>,
    pub operation: Option<Operations>,
}

impl<'a> From<&'a MetaTransactionData> for EstimateRequest<'a> {
    fn from(val: &'a MetaTransactionData) -> Self {
        EstimateRequest {
            to: val.to.into(),
            value: val.value,
            data: val.data.as_ref(),
            operation: val.operation.unwrap_or(Operations::Call),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SafeGasConfig {
    /// Gas to be forwarded to the callee. 0 for all available
    pub safe_tx_gas: u64,
    /// Gas cost that is independent of the internal transaction execution,
    /// (e.g. base transaction fee, signature check, payment of the refund)
    pub base_gas: u64,
    /// Maximum gas price that should be used for this transaction. 0 for no
    /// maximum. For base layer tokens, (e.g. ETH), this is adjusted to be no
    /// higher than that actual gas price used. For custom refund tokens, it
    /// may be any amount.
    pub gas_price: u64,
    /// Token address (or 0 if ETH) that is used for the reimbursement payment
    /// to the executor.
    pub gas_token: ChecksumAddress,
    /// The address which receives the refund. Defaults to `tx.origin` if empty
    pub refund_receiver: ChecksumAddress,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeTransactionData {
    #[serde(flatten)]
    pub core: MetaTransactionData,
    #[serde(flatten)]
    pub gas: SafeGasConfig,
    /// The Safe nonce to use
    pub nonce: u64,
}

impl<'a> From<&'a SafeTransactionData> for EstimateRequest<'a> {
    fn from(val: &'a SafeTransactionData) -> Self {
        From::from(&val.core)
    }
}

// Internal type to support 712 trait impl
#[derive(Clone, Debug)]
pub struct SafeEip712<'a> {
    address: Address,
    chain_id: u64,
    tx: &'a SafeTransactionData,
}

impl<'a> Eip712 for SafeEip712<'a> {
    type Error = Infallible;

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: None,
            version: None,
            chain_id: Some(self.chain_id.into()),
            verifying_contract: Some(self.address),
            salt: None,
        })
    }

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        Ok(From::from(*SAFE_TX_TYPEHASH))
    }

    fn struct_hash(&self) -> Result<[u8; 32], Self::Error> {
        Ok(keccak256(self.tx.encode_struct()))
    }

    fn encode_eip712(&self) -> Result<[u8; 32], Self::Error> {
        // encode the digest to be compatible with solidity abi.encodePacked()
        // See: https://github.com/gakonst/ethers-rs/blob/master/examples/permit_hash.rs#L72

        let domain_separator = self.domain_separator()?;
        let struct_hash = self.struct_hash()?;

        let digest_input = [&[0x19, 0x01], &domain_separator[..], &struct_hash[..]].concat();

        Ok(keccak256(digest_input))
    }

    fn domain_separator(&self) -> Result<[u8; 32], Self::Error> {
        let mut encoded = [0u8; 96];
        encoded[..32].copy_from_slice(DOMAIN_SEPARATOR_TYPEHASH.as_fixed_bytes());
        U256::from(self.chain_id).to_big_endian(&mut encoded[32..64]);
        encoded[64 + 12..].copy_from_slice(self.address.as_bytes());
        Ok(keccak256(&encoded))
    }
}

impl Tokenize for &SafeTransactionData {
    fn into_tokens(self) -> Vec<ethers::abi::Token> {
        let data = H256::from(keccak256(&self.core.data.as_deref().unwrap_or(&[])));
        (
            *SAFE_TX_TYPEHASH,
            self.core.to,
            self.core.value,
            data,
            self.core.operation.unwrap_or(Operations::Call),
            self.gas.safe_tx_gas,
            self.gas.base_gas,
            self.gas.gas_price,
            self.gas.gas_token,
            self.gas.refund_receiver,
            self.nonce,
        )
            .into_tokens()
    }
}

impl SafeTransactionData {
    pub fn eip712(&self, safe_address: Address, chain_id: u64) -> SafeEip712 {
        SafeEip712 {
            address: safe_address,
            chain_id,
            tx: self,
        }
    }

    fn encode_struct(&self) -> Vec<u8> {
        abi::encode(&self.into_tokens())
    }

    fn encode_eip712(&self, safe_address: Address, chain_id: u64) -> H256 {
        self.eip712(safe_address, chain_id)
            .encode_eip712()
            .unwrap()
            .into()
    }

    /// Sign the safe transaction hash
    async fn sign<S: Signer>(
        &self,
        signer: &S,
        safe_address: Address,
        chain_id: u64,
    ) -> Result<ProposeSignature, S::Error> {
        let eip712 = SafeEip712 {
            address: safe_address,
            chain_id,
            tx: self,
        };
        let signature = signer.sign_typed_data(&eip712).await?;
        Ok(ProposeSignature {
            sender: signer.address().into(),
            signature,
            origin: None,
        })
    }

    /// Sign the afe Transaction hash and create a safe transaction service
    /// Propose request
    pub async fn into_request<S: Signer>(
        self,
        signer: &S,
        safe_address: Address,
        chain_id: u64,
    ) -> Result<ProposeRequest, S::Error> {
        let signature = self.sign(signer, safe_address, chain_id).await?;
        let contract_transaction_hash = self.encode_eip712(safe_address, chain_id);
        Ok(ProposeRequest {
            tx: self,
            contract_transaction_hash,
            signature,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProposeSignature {
    sender: ChecksumAddress,
    #[serde(with = "rsv_sig_ser")]
    /// Must be in RSV format
    signature: Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
}

mod rsv_sig_ser {
    use ethers::types::Signature;
    use serde::{Deserialize, Serialize};

    pub(crate) fn serialize<S>(sig: &Signature, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        sig.to_string().serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Signature, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse::<Signature>()
            .map_err(serde::de::Error::custom)
    }
}

impl ProposeSignature {
    /// Getter for `signature`
    pub fn signature(&self) -> Signature {
        self.signature
    }

    /// Getter for `sender`
    pub fn sender(&self) -> ChecksumAddress {
        self.sender
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProposeRequest {
    #[serde(flatten)]
    pub(crate) tx: SafeTransactionData,
    pub(crate) contract_transaction_hash: H256,
    #[serde(flatten)]
    pub(crate) signature: ProposeSignature,
}

impl ProposeRequest {
    pub fn url(root: &Url, address: impl Into<ChecksumAddress>) -> Url {
        let path = format!("api/v1/safes/{}/multisig-transactions/", address.into());
        let mut url = root.clone();
        url.set_path(&path);
        url
    }

    /// Returns the Safe's internal tx hash for this request
    pub fn contract_transaction_hash(&self) -> H256 {
        self.contract_transaction_hash
    }
    /// Alias for contract_transaction_hash
    pub fn safe_tx_hash(&self) -> H256 {
        self.contract_transaction_hash
    }

    /// Returns the TX details for this request
    pub fn tx(&self) -> &SafeTransactionData {
        &self.tx
    }

    /// Returns the signature details for this request
    pub fn signature(&self) -> &ProposeSignature {
        &self.signature
    }
}
