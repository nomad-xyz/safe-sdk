use std::convert::Infallible;

use ethers::{
    abi::Tokenize,
    signers::Signer,
    types::{
        transaction::eip712::{EIP712Domain, Eip712},
        Address, Bytes, Signature, H256, U256,
    },
    utils::keccak256,
};
use reqwest::Url;

use crate::rpc::common::{Operations, DOMAIN_SEPARATOR_TYPEHASH};

use super::{common::SAFE_TX_TYPEHASH, estimate::EstimateRequest};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MetaTransactionData {
    to: Address,
    value: U256,
    data: Bytes,
    operation: Option<Operations>,
}

impl<'a> From<&'a MetaTransactionData> for EstimateRequest<'a> {
    fn from(val: &'a MetaTransactionData) -> Self {
        EstimateRequest {
            to: val.to,
            value: val.value.low_u64(),
            data: &val.data,
            operation: val.operation.unwrap_or(Operations::Call),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SafeTransactionData {
    #[serde(flatten)]
    core: MetaTransactionData,
    safe_tx_gas: U256,
    base_gas: U256,
    gas_price: U256,
    gas_token: Address,
    refund_receiver: Address,
    nonce: U256,
}

impl<'a> From<&'a SafeTransactionData> for EstimateRequest<'a> {
    fn from(val: &'a SafeTransactionData) -> Self {
        From::from(&val.core)
    }
}

// Internal type to support 712 trait impl
#[derive(Clone, Debug)]
struct SafeEip712<'a> {
    address: Address,
    chain_id: U256,
    tx: &'a SafeTransactionData,
}

impl<'a> Eip712 for SafeEip712<'a> {
    type Error = Infallible;

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: None,
            version: None,
            chain_id: Some(self.chain_id),
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
}

impl SafeTransactionData {
    /// Calculate the eip712 domain separator, which commits to the verifier
    /// address, and the chain_id
    pub fn domain_separator(&self, safe_address: Address, chain_id: U256) -> H256 {
        let mut encoded = [0u8; 96];
        encoded[..32].copy_from_slice(DOMAIN_SEPARATOR_TYPEHASH.as_fixed_bytes());
        chain_id.to_big_endian(&mut encoded[32..64]);
        encoded[64 + 12..].copy_from_slice(safe_address.as_bytes());

        let tokens = (*DOMAIN_SEPARATOR_TYPEHASH, chain_id, safe_address).into_tokens();
        keccak256(ethers::abi::encode(&tokens)).into()
    }

    /// Encode this struct suitable for EIP-712 signing
    pub fn encode_struct(&self) -> Vec<u8> {
        let tokens = (
            *SAFE_TX_TYPEHASH,
            self.core.to,
            self.core.value,
            H256::from(keccak256(&self.core.data)),
            self.core.operation.unwrap_or(Operations::Call) as u8,
            self.safe_tx_gas,
            self.base_gas,
            self.gas_price,
            self.gas_token,
            self.refund_receiver,
            self.nonce,
        )
            .into_tokens();
        ethers::abi::encode(&tokens)
    }

    /// Calculate the EIP712 struct hash
    fn struct_hash(&self, safe_address: Address, chain_id: U256) -> H256 {
        SafeEip712 {
            address: safe_address,
            chain_id,
            tx: self,
        }
        .struct_hash()
        .unwrap()
        .into()
    }

    /// Sign the safe transaction hash
    async fn sign<S: Signer>(
        &self,
        signer: &S,
        safe_address: Address,
        chain_id: U256,
    ) -> Result<ProposeSignature, S::Error> {
        let eip712 = SafeEip712 {
            address: safe_address,
            chain_id,
            tx: self,
        };
        let signature = signer.sign_typed_data(&eip712).await?;
        Ok(ProposeSignature {
            sender: signer.address(),
            signature,
            origin: None,
        })
    }

    /// Sign the afe Transaction hash and create a safe transaction service
    /// Propose request
    pub async fn to_request<S: Signer>(
        &self,
        signer: &S,
        safe_address: Address,
        chain_id: U256,
    ) -> Result<ProposeRequest<'_>, S::Error> {
        let signature = self.sign(signer, safe_address, chain_id).await?;
        let contract_transaction_hash = self.struct_hash(safe_address, chain_id);
        Ok(ProposeRequest {
            tx: self,
            contract_transaction_hash,
            signature,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ProposeSignature {
    sender: Address,
    /// Must be in RSV format
    signature: Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
}

impl ProposeSignature {
    /// Getter for `signature`
    pub fn signature(&self) -> Signature {
        self.signature
    }

    /// Getter for `sender`
    pub fn sender(&self) -> Address {
        self.sender
    }
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ProposeRequest<'a> {
    #[serde(flatten)]
    tx: &'a SafeTransactionData,
    contract_transaction_hash: H256,
    #[serde(flatten)]
    signature: ProposeSignature,
}

impl<'a> ProposeRequest<'a> {
    pub fn url(root: &Url, safe_address: Address) -> Url {
        let path = format!("v1/safes/{:?}/multisig-transactions/", safe_address);
        let mut url = root.clone();
        url.set_path(&path);
        url
    }
}
