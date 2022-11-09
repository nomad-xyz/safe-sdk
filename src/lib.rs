mod macros;

pub mod client;
// pub mod propose;

pub mod rpc;

pub use client::{ClientError, GnosisClient};

// currently supported:
// `/v1/safes/{address}`
// `/v1/safes/{address}/multisig-transactions`
