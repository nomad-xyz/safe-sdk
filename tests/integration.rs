use ethers::{
    signers::{LocalWallet, Signer},
    types::Address,
};
use gnosis_sdk::{
    client::SigningClient,
    rpc::{
        common::{ChecksumAddress, Operations},
        propose::MetaTransactionData,
    },
};
use once_cell::sync::Lazy;

pub const KEY: &str = "1c3a7cdd2270579847aaec11680312cbf4d3c36886232b413ab6529593228ec2";
pub const ADDRESS: &str = "0xD5F586B9b2abbbb9a9ffF936690A54F9849dbC97";
pub const SAFE_ADDRESS: &str = "0x38CD8Fa77ECEB4b1edB856Ed27aac6A6c6Dc88ca";

pub static WALLET: Lazy<LocalWallet> = Lazy::new(|| {
    let wallet: LocalWallet = KEY.parse().unwrap();
    wallet.with_chain_id(5u64)
});
pub static ADDR: Lazy<Address> = Lazy::new(|| ADDRESS.parse().unwrap());
pub static SAFE: Lazy<Address> = Lazy::new(|| SAFE_ADDRESS.parse().unwrap());

#[tokio::test]
#[tracing_test::traced_test]
async fn it_gets_info() {
    let client = SigningClient::try_from_signer(WALLET.clone()).unwrap();
    client.safe_info(*SAFE).await.unwrap();
}

#[tokio::test]
#[tracing_test::traced_test]
async fn it_proposes() {
    let client = SigningClient::try_from_signer(WALLET.clone()).unwrap();
    let tx: MetaTransactionData = MetaTransactionData {
        to: ChecksumAddress::from(*ADDR),
        value: 381832418u64,
        data: Some("0xdeadbeefdeadbeef".parse().unwrap()),
        operation: Some(Operations::DelegateCall),
    };
    dbg!(client.propose(tx, *SAFE).await.unwrap());
}
