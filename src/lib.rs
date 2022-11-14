#![warn(missing_docs, unreachable_pub)]
#![deny(unused_must_use, rust_2018_idioms)]

//! Safe Transaction Service SDK

mod macros;

/// RPC Client & Signing Client
pub mod client;

/// RPC method structs
pub mod rpc;

/// ethers middleware
pub mod middleware;

/// Network configuration
pub mod networks;

pub use client::{ClientError, SafeClient, SigningClient, SigningClientError};

// currently supported:
// GET `/v1/safes/{address}`
// GET `/v1/safes/{address}/multisig-transactions`
// POST `/v1/safes/{address}/multisig-transactions`
// POST `/v1/safes/{:?}/multisig-transactions/estimations/`
