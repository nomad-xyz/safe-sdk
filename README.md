# Safe Transaction Service API Client

## Using the SDK

### Instantiate an API client

```rust
use safe_sdk::SafeClient;

/// From a chain id, by looking up hardcoded endpoints
let client = SafeClient::by_chain_id(1);

/// From mainnet ethereum
let client = SafeClient::ethereum();

/// From an endpoint/chain ID pair
let service = safe_sdk::networks::TxService { url: "". chain_id: 0};
let client = SafeClient::new(service);
```

### Instantiate a signing client

```rust
use safe_sdk::SigningClient;

/// From an ethers signer
let client = SigningClient::try_from_signer(ethers_signer);

/// From ethereum with an ethers signer
/// overrides the chain_id of the signer
let client = SigningClient::ethereum(ethers_signer);

/// From a service and signer
let service = safe_sdk::networks::TxService { url: "". chain_id: 0};
let client = SigningClient::with_service_and_signer(service, ethers_signer);

/// From an existing SafeClient
let client = safe_client.with_signer(ethers_signer);
```

### Common Safe Actions

```rust
use safe_sdk::rpc::info::SafeInfoResponse;

/// Read info
let info: SafeInfoResponse = client.safe_info(safe_address).await?;
dbg!(&info.nonce); // u64 of on-chain Nonce
dbg!(&info.owners) // vec of addresses

/// Get next available nonce
let next_nonce = client.next_nonce(safe_address).await?;

/// Get SAFE msig tx history
let history = client.msig_history_builder().query(safe_address).await?;

/// Get SAFE msig tx history by nonce range
let history = client.msig_history_builder()
    .min_nonce(15)
    .max_nonce(25)
    .query(safe_address)
    .await?;
```

### Dispatch

```rust
let tx =
```

### TODOs & Rough Edges

- Most endpoints are not implemented yet. This SDK prioritizes automated TX
  submission, NOT full API implementation.
- Many params/types are not yet implemented.
- API documentation is incomplete and we don't know the function of some
  properties.
- Better integration with ethers
  - More implementations of `From<X> for MetaTransactionData`
- Some properties are stringly typed, and should be turned into enums
- Refine the API Response type
