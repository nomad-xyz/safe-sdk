mod macros;

pub mod client;

pub mod rpc;

/// ethers middleware
pub mod middleware;

pub use client::{ClientError, GnosisClient};

// currently supported:
// GET `/v1/safes/{address}`
// GET `/v1/safes/{address}/multisig-transactions`
// POST `/v1/safes/{address}/multisig-transactions`
// POST `/v1/safes/{:?}/multisig-transactions/estimations/`
