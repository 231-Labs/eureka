/// Devnet
pub const EUREKA_DEVNET_PACKAGE_ID: &str = "0xa73e8ca153ab4f98b212ac1ee21d69a9d2c7bfacf9ef0d63326bfe72f47727ba";
pub const EUREKA_DEVNET_PRINTER_REGISTRY_ID: &str = "0xbd50616301535dcb649ddfc628a7163527def73a97cb387b09c5f4352285ca86";
pub const SCULPT_DEVNET_PACKAGE_ID: &str = "0x2571c1e364b5647e1ee17b43f9f289e5c64ce3a0c38f6f9441a3f331e0083efa";

/// Testnet
pub const WALRUS_COIN_TYPE: &str = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL";
pub const EUREKA_TESTNET_PACKAGE_ID: &str = "0xdf87f76e34fb02000a00fd6a58e5d7b5e1f1d76b1a6399ff7079cf7c9991bd2a";
pub const EUREKA_TESTNET_PRINTER_REGISTRY_ID: &str = "0x4aefe6483a8bbc5258b7668c867291581800da1d7c913c923a2c64a3beecfc3c";
pub const SCULPT_TESTNET_PACKAGE_ID: &str = "0xdf87f76e34fb02000a00fd6a58e5d7b5e1f1d76b1a6399ff7079cf7c9991bd2a";
pub const AGGREGATOR_URL: &str = "https://walrus-agg-test.bucketprotocol.io";

// Global constants
pub const GAS_BUDGET: u64 = 100_000_000;

#[allow(dead_code)]
pub const SUI_CLOCK_OBJECT_ID: &str = "0x6";

pub struct NetworkPackageIds {
    pub eureka_package_id: &'static str,
    pub eureka_printer_registry_id: &'static str,
    pub bottega_package_id: &'static str,
}

pub const NETWORK_PACKAGE_IDS: [NetworkPackageIds; 3] = [
    NetworkPackageIds {
        eureka_package_id: EUREKA_DEVNET_PACKAGE_ID,
        eureka_printer_registry_id: EUREKA_DEVNET_PRINTER_REGISTRY_ID,
        bottega_package_id: SCULPT_DEVNET_PACKAGE_ID,
    },
    NetworkPackageIds {
        eureka_package_id: EUREKA_TESTNET_PACKAGE_ID,
        eureka_printer_registry_id: EUREKA_TESTNET_PRINTER_REGISTRY_ID,
        bottega_package_id: SCULPT_TESTNET_PACKAGE_ID,
    },
    NetworkPackageIds {
        eureka_package_id: "",  // TODO: Add mainnet ID
        eureka_printer_registry_id: "",
        bottega_package_id: "",
    },
];

pub const NETWORKS: [(&str, &str); 3] = [
    ("devnet", "https://fullnode.devnet.sui.io:443"),
    ("testnet", "https://fullnode.testnet.sui.io:443"),
    ("mainnet", "https://fullnode.mainnet.sui.io:443"),
];