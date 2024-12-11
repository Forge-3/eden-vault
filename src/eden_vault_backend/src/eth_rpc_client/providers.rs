use evm_rpc_client::RpcService as EvmRpcService;

pub(crate) const MAINNET_PROVIDERS: [RpcNodeProvider; 4] = [
    RpcNodeProvider::Ethereum(EthereumProvider::BlockPi),
    RpcNodeProvider::Ethereum(EthereumProvider::PublicNode),
    RpcNodeProvider::Ethereum(EthereumProvider::LlamaNodes),
    RpcNodeProvider::Ethereum(EthereumProvider::Alchemy),
];

pub(crate) const SEPOLIA_PROVIDERS: [RpcNodeProvider; 4] = [
    RpcNodeProvider::Sepolia(SepoliaProvider::BlockPi),
    RpcNodeProvider::Sepolia(SepoliaProvider::PublicNode),
    RpcNodeProvider::Sepolia(SepoliaProvider::Alchemy),
    RpcNodeProvider::Sepolia(SepoliaProvider::RpcSepolia),
];

pub(crate) const LOCAL_PROVIDERS: [RpcNodeProvider; 1] =
    [RpcNodeProvider::Local(LocalService::Local)];

pub(crate) const BSC_PROVIDERS: [RpcNodeProvider; 1] = [
    RpcNodeProvider::BSC(BSCService::BlockPi),
];

pub(crate) const BSC_TESTNET_PROVIDERS: [RpcNodeProvider; 2] = [
    RpcNodeProvider::BSCTestnet(BSCTestnetService::BlockPi),
    RpcNodeProvider::BSCTestnet(BSCTestnetService::PublicNode),
];

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(crate) enum RpcNodeProvider {
    Ethereum(EthereumProvider),
    Sepolia(SepoliaProvider),
    EvmRpc(EvmRpcService),
    Local(LocalService),
    BSC(BSCService),
    BSCTestnet(BSCTestnetService),
}

impl RpcNodeProvider {
    //TODO XC-27: remove this method
    pub(crate) fn url(&self) -> &str {
        match self {
            Self::Ethereum(provider) => provider.ethereum_mainnet_endpoint_url(),
            Self::Sepolia(provider) => provider.ethereum_sepolia_endpoint_url(),
            Self::Local(provider) => provider.local_evm_endpoint_url(),
            Self::BSC(provider) => provider.bsc_mainnet_endpoint_url(),
            Self::BSCTestnet(provider) => provider.bsc_testnet_endpoint_url(),
            RpcNodeProvider::EvmRpc(_) => {
                panic!("BUG: should not need URL of provider from EVM RPC canister")
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum EthereumProvider {
    // https://blockpi.io/
    BlockPi,
    // https://publicnode.com/
    PublicNode,
    // https://llamanodes.com/
    LlamaNodes,
    Alchemy,
}

impl EthereumProvider {
    fn ethereum_mainnet_endpoint_url(&self) -> &str {
        match self {
            EthereumProvider::BlockPi => "https://ethereum.blockpi.network/v1/rpc/public",
            EthereumProvider::PublicNode => "https://ethereum-rpc.publicnode.com",
            EthereumProvider::LlamaNodes => "https://eth.llamarpc.com",
            EthereumProvider::Alchemy => "https://eth-mainnet.g.alchemy.com/v2/demo",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum SepoliaProvider {
    // https://blockpi.io/
    BlockPi,
    // https://publicnode.com/
    PublicNode,
    // https://www.alchemy.com/chain-connect/endpoints/rpc-sepolia-sepolia
    Alchemy,
    RpcSepolia,
}

impl SepoliaProvider {
    fn ethereum_sepolia_endpoint_url(&self) -> &str {
        match self {
            SepoliaProvider::BlockPi => "https://ethereum-sepolia.blockpi.network/v1/rpc/public",
            SepoliaProvider::PublicNode => "https://ethereum-sepolia-rpc.publicnode.com",
            SepoliaProvider::Alchemy => "https://eth-sepolia.g.alchemy.com/v2/demo",
            SepoliaProvider::RpcSepolia => "https://rpc.sepolia.org",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum BSCTestnetService {
    // https://bsc-testnet.blockpi.network/v1/rpc/public
    BlockPi,
    // https://bsc-testnet-rpc.publicnode.com
    PublicNode,
}

impl BSCTestnetService {
    fn bsc_testnet_endpoint_url(&self) -> &str {
        match self {
            BSCTestnetService::BlockPi => "https://bsc-testnet.blockpi.network/v1/rpc/public",
            BSCTestnetService::PublicNode => "https://bsc-testnet-rpc.publicnode.com",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum BSCService {
    // https://bsc-mainnet.public.blastapi.io
    BlastApi,
    // https://bsc.blockpi.network/v1/rpc/public
    BlockPi,
    // https://bsc.drpc.org
    Drpc,
    // https://bsc-rpc.publicnode.com
    PublicNode,
    // https://binance.llamarpc.com
    LlamaRpc
}

impl BSCService {
    fn bsc_mainnet_endpoint_url(&self) -> &str {
        match self {
            BSCService::BlastApi => "https://bsc-mainnet.public.blastapi.io",
            BSCService::BlockPi => "https://bsc.blockpi.network/v1/rpc/public",
            BSCService::Drpc => "https://bsc.drpc.org",
            BSCService::PublicNode => "https://bsc-rpc.publicnode.com",
            BSCService::LlamaRpc => "https://binance.llamarpc.com",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum LocalService {
    // http://127.0.0.1:8545/
    Local,
}

impl LocalService {
    fn local_evm_endpoint_url(&self) -> &str {
        match self {
            LocalService::Local => "http://127.0.0.1:8545",
        }
    }
}
