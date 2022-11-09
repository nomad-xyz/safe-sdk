use std::fmt::Display;

use ethers::types::H256;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::client::ClientResult;

/// EIP-712 Tx Details typehash. Copied from gnosis safe contracts
///
/// keccak256(
///     "SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)"
/// );
pub static SAFE_TX_TYPEHASH: Lazy<H256> = Lazy::new(|| {
    "0xbb8310d486368db6bd6f849402fdd73ad53d316b5a4b2644ad6efe0f941286d8"
        .parse()
        .unwrap()
});

/// EIP-712 typehash domain binding. Copied from gnosis safe contracts
///
/// keccak256(
///     "EIP712Domain(uint256 chainId,address verifyingContract)"
/// );
pub static DOMAIN_SEPARATOR_TYPEHASH: Lazy<H256> = Lazy::new(|| {
    "0x47e79534a245952e8b16893a336b85a3d9ea9fa8c573f3d803afb92a79469218"
        .parse()
        .unwrap()
});

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operations {
    Call = 0,
    DelegateCall = 1,
}

impl Serialize for Operations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (*self as u8).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Operations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        u8::deserialize(deserializer).map(|num| {
            if num == 2 {
                Operations::DelegateCall
            } else {
                Operations::Call
            }
        })
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ErrorResponse {
    pub code: u8,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub arguments: Vec<serde_json::Value>,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Code: {}, Mesage: \"{}\"",
            self.code,
            self.message.as_deref().unwrap_or(""),
        ))
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    Error(ErrorResponse),
    Sucess(T),
}

impl<T> ApiResponse<T> {
    pub(crate) fn into_client_result(self) -> ClientResult<T> {
        match self {
            ApiResponse::Error(e) => Err(e.into()),
            ApiResponse::Sucess(t) => Ok(t),
        }
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SafeVersions {
    V3,
    V4,
}

impl serde::Serialize for SafeVersions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let v = match self {
            SafeVersions::V3 => "v3",
            SafeVersions::V4 => "v4",
        };
        serializer.serialize_str(v)
    }
}
