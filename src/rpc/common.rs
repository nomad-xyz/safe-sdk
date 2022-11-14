use std::{fmt::Display, str::FromStr};

use ethers::{
    abi::{ethereum_types::FromDecStrErr, InvalidOutputType, Token, Tokenizable},
    types::{Address, Bytes, H256, U256},
};
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
/// EIP712 supports several forms of domain binding, but only veriifer and
/// chain id are used here. See [`ethers::types::transaction::EIP712Domain`]
///
/// keccak256(
///     "EIP712Domain(uint256 chainId,address verifyingContract)"
/// );
pub static DOMAIN_SEPARATOR_TYPEHASH: Lazy<H256> = Lazy::new(|| {
    "0x47e79534a245952e8b16893a336b85a3d9ea9fa8c573f3d803afb92a79469218"
        .parse()
        .unwrap()
});

pub(crate) fn default_empty_bytes_ref<S>(
    bytes: &Option<&Bytes>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match bytes {
        Some(buf) => buf.serialize(serializer),
        None => Bytes::default().serialize(serializer),
    }
}

pub(crate) fn default_empty_bytes<S>(
    bytes: &Option<Bytes>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match bytes {
        Some(buf) => buf.serialize(serializer),
        None => Bytes::default().serialize(serializer),
    }
}

/// Safe operations
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operations {
    /// CALL opcode
    Call = 0,
    /// DELEGATECALL opcode.
    /// Note: please exercise caution, as this can brick a SAFE
    DelegateCall = 1,
}

impl Tokenizable for Operations {
    fn from_token(token: ethers::abi::Token) -> Result<Self, ethers::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        match token {
            Token::Uint(x) if x.is_zero() => Ok(Operations::Call),
            Token::Uint(x) if x == U256::from(1) => Ok(Operations::DelegateCall),
            other => Err(InvalidOutputType(format!("Expected 0 or 1, got {}", other))),
        }
    }

    fn into_token(self) -> ethers::abi::Token {
        match self {
            Operations::Call => Token::Uint(0.into()),
            Operations::DelegateCall => Token::Uint(1.into()),
        }
    }
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

/// API Error response
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ErrorResponse {
    /// Error code
    pub code: u8,
    /// Error message
    #[serde(default)]
    pub message: Option<String>,
    /// Inputs
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

/// API Response
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    /// Error
    Error(ErrorResponse),
    /// Success w/ value
    Success(T),
    /// Empty Success
    EmptySuccess,
}

impl<T> FromStr for ApiResponse<T>
where
    T: serde::de::DeserializeOwned,
{
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(ApiResponse::EmptySuccess);
        }
        serde_json::from_str(s)
    }
}

impl<T> ApiResponse<T> {
    pub(crate) fn into_client_result(self) -> ClientResult<Option<T>> {
        match self {
            ApiResponse::Error(e) => Err(e.into()),
            ApiResponse::Success(t) => Ok(Some(t)),
            ApiResponse::EmptySuccess => Ok(None),
        }
    }

    /// True if the response is an API error
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Safe versions
/// TODO: what do these do?
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SafeVersions {
    /// V3
    V3,
    /// V4
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

/// An address wrapper that ensures checksum encoding
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct ChecksumAddress(pub Address);

impl std::ops::Deref for ChecksumAddress {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Address> for ChecksumAddress {
    fn from(addr: Address) -> Self {
        Self(addr)
    }
}

impl From<ChecksumAddress> for Address {
    fn from(val: ChecksumAddress) -> Self {
        val.0
    }
}

impl serde::Serialize for ChecksumAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ethers::utils::to_checksum(self, None).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ChecksumAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Address::deserialize(deserializer)?.into())
    }
}

impl std::fmt::Debug for ChecksumAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ethers::utils::to_checksum(self, None))
    }
}

impl std::fmt::Display for ChecksumAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ethers::utils::to_checksum(self, None))
    }
}

impl FromStr for ChecksumAddress {
    type Err = <Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Address>().map(Into::into)
    }
}

impl ethers::abi::Tokenizable for ChecksumAddress {
    fn from_token(token: ethers::abi::Token) -> Result<Self, ethers::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        Address::from_token(token).map(Into::into)
    }

    fn into_token(self) -> ethers::abi::Token {
        self.0.into_token()
    }
}

/// A U256 wrapper that ensures decimal string encoding
#[derive(Debug, Clone, Copy, Default)]
pub struct DecimalU256(U256);

impl std::ops::Deref for DecimalU256 {
    type Target = U256;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<U256> for DecimalU256 {
    fn from(i: U256) -> Self {
        Self(i)
    }
}

impl From<DecimalU256> for U256 {
    fn from(i: DecimalU256) -> Self {
        i.0
    }
}

impl FromStr for DecimalU256 {
    type Err = FromDecStrErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(U256::from_dec_str(s)?.into())
    }
}

impl serde::Serialize for DecimalU256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{}", self.0).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for DecimalU256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use crate::rpc::info::SafeInfoResponse;

    #[test]
    fn it_does() {
        let resp = "{\"address\":\"0x38CD8Fa77ECEB4b1edB856Ed27aac6A6c6Dc88ca\",\"nonce\":0,\"threshold\":2,\"owners\":[\"0xD5F586B9b2abbbb9a9ffF936690A54F9849dbC97\",\"0x425249Cf0F2f91f488E24cF7B1AA3186748f7516\"],\"masterCopy\":\"0x3E5c63644E683549055b9Be8653de26E0B4CD36E\",\"modules\":[],\"fallbackHandler\":\"0xf48f2B2d2a534e402487b3ee7C18c33Aec0Fe5e4\",\"guard\":\"0x0000000000000000000000000000000000000000\",\"version\":\"1.3.0+L2\"}";

        let _: super::ApiResponse<SafeInfoResponse> = serde_json::from_str(resp).unwrap();
    }
}
