use ethers::{
    signers::{LocalWallet, Signer},
    types::{Address, H256},
};
use once_cell::sync::Lazy;
use safe_sdk::{
    client::SigningClient,
    rpc::{
        common::{ChecksumAddress, Operations},
        propose::MetaTransactionData,
    },
};

pub const KEY: &str = "1c3a7cdd2270579847aaec11680312cbf4d3c36886232b413ab6529593228ec2";
pub const ADDRESS: &str = "0xD5F586B9b2abbbb9a9ffF936690A54F9849dbC97";
// https://gnosis-safe.io/app/gor:0x38CD8Fa77ECEB4b1edB856Ed27aac6A6c6Dc88ca/home
pub const SAFE_ADDRESS: &str = "0x38CD8Fa77ECEB4b1edB856Ed27aac6A6c6Dc88ca";

pub static WALLET: Lazy<LocalWallet> = Lazy::new(|| {
    let wallet: LocalWallet = KEY.parse().unwrap();
    wallet.with_chain_id(5u64)
});
pub static ADDR: Lazy<Address> = Lazy::new(|| ADDRESS.parse().unwrap());
pub static SAFE: Lazy<Address> = Lazy::new(|| SAFE_ADDRESS.parse().unwrap());

pub const DOMAIN_SEPARATOR: &str =
    "0x647732b0b00d304899db2afe3fb46661547fd844fe5a32e337b32ebf4d141839";
pub static SEPARATOR: Lazy<H256> = Lazy::new(|| DOMAIN_SEPARATOR.parse().unwrap());

pub const GOERLI_CHAIN_ID: u64 = 5;

pub static CLIENT: Lazy<SigningClient<LocalWallet>> =
    Lazy::new(|| SigningClient::try_from_signer(WALLET.clone()).unwrap());

#[tokio::test]
#[tracing_test::traced_test]
async fn it_gets_info() {
    CLIENT.safe_info(*SAFE).await.unwrap();
}

#[tokio::test]
#[tracing_test::traced_test]
async fn it_gets_history() {
    dbg!(CLIENT.msig_history(*SAFE).await.unwrap());
}

#[tokio::test]
#[tracing_test::traced_test]
async fn it_proposes() {
    let tx: MetaTransactionData = MetaTransactionData {
        to: ChecksumAddress::from(*ADDR),
        value: 381832418u64,
        data: Some("0xdeadbeefdeadbeef".parse().unwrap()),
        operation: Some(Operations::DelegateCall),
    };

    dbg!(CLIENT.propose(tx, *SAFE).await.unwrap().nonce);
}
