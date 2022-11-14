/// Safe Transaction Service details
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TxService {
    /// URL of the service
    pub url: &'static str,
    /// Chain id of the network
    pub chain_id: u64,
}

impl TxService {
    /// Const constructor :)
    pub const fn new(url: &'static str, chain_id: u64) -> Self {
        Self { url, chain_id }
    }

    /// Runtime Lookup
    pub fn by_chain_id(chain_id: u64) -> Option<Self> {
        SERVICES
            .iter()
            .find(|service| service.chain_id == chain_id)
            .copied()
    }
}

/// ETHEREUM
pub const ETHEREUM: TxService = TxService::new("https://safe-transaction-mainnet.safe.global/", 1);
/// XDAI
pub const XDAI: TxService = TxService::new("https://safe-transaction.xdai.gnosis.io/", 100);
/// ARBITRUM
pub const ARBITRUM: TxService =
    TxService::new("https://safe-transaction.arbitrum.gnosis.io/", 42151);
/// const
pub const AVALANCHE: TxService =
    TxService::new("https://safe-transaction.avalanche.gnosis.io/", 43114);
/// const
pub const AURORA: TxService =
    TxService::new("https://safe-transaction-aurora.safe.global", 1313161554);
/// const
pub const BSC: TxService = TxService::new("https://safe-transaction-bsc.safe.global", 56);

/// OPTIMISM
pub const OPTIMISM: TxService = TxService::new("https://safe-transaction-optimism.safe.global", 10);
/// POLYGON
pub const POLYGON: TxService = TxService::new("https://safe-transaction-polygon.safe.global", 137);
/// GOERLI
pub const GOERLI: TxService = TxService::new("https://safe-transaction-goerli.safe.global", 5);
// the heck is an energy web chain smdh
/// EWC
pub const EWC: TxService = TxService::new("https://safe-transaction-ewc.safe.global", 246);
/// VOLTA
pub const VOLTA: TxService = TxService::new("https://safe-transaction-volta.safe.global", 73799);

/// GNOSIS_CHAIN (alias for XDAI)
pub const GNOSIS_CHAIN: TxService = XDAI;
/// BINANCE_SMART_CHAIN (alias for BSC)
pub const BINANCE_SMART_CHAIN: TxService = BSC;

/// Iterable, deduplicated list of known services
pub const SERVICES: &[TxService] = &[
    ETHEREUM, XDAI, ARBITRUM, AVALANCHE, AURORA, BSC, OPTIMISM, POLYGON, GOERLI, EWC, VOLTA,
];
