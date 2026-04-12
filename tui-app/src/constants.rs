/// Devnet
pub const EUREKA_DEVNET_PACKAGE_ID: &str = "0xa73e8ca153ab4f98b212ac1ee21d69a9d2c7bfacf9ef0d63326bfe72f47727ba";
pub const EUREKA_DEVNET_PRINTER_REGISTRY_ID: &str = "0xbd50616301535dcb649ddfc628a7163527def73a97cb387b09c5f4352285ca86";
pub const SCULPT_DEVNET_PACKAGE_ID: &str = "0x2571c1e364b5647e1ee17b43f9f289e5c64ce3a0c38f6f9441a3f331e0083efa";

/// Testnet — `SCULPT_TESTNET_PACKAGE_ID` must match `archimeters-1/contract/Published.toml` → `[published.testnet].published-at`.
/// Eureka package / registry from fresh publish (digest `9r5844hQmGrNrRQkD8gTKHL4A4jFW9imwQ1aLGtULVZ9`).
pub const WALRUS_COIN_TYPE: &str = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL";
pub const EUREKA_TESTNET_PACKAGE_ID: &str =
    "0x1737bb093b90783dfe0e0056df602bdfa42fc417d91fed1e02a27a88b949c3b3";
pub const EUREKA_TESTNET_PRINTER_REGISTRY_ID: &str =
    "0x3498e9fef83b29ef471d3070daf7764f3f9abcc982daa34fdf7fda9b612e9409";
pub const SCULPT_TESTNET_PACKAGE_ID: &str = "0x51d9c918431258ae6748b50234d0da3d436e6df8e2087fa1446913e390336ab8";
pub const AGGREGATOR_URL: &str = "https://walrus-agg-test.bucketprotocol.io";

// Global constants
pub const GAS_BUDGET: u64 = 100_000_000;

#[allow(dead_code)]
pub const SUI_CLOCK_OBJECT_ID: &str = "0x6";

pub const PRINT_JOB_POLL_INTERVAL_SECS: u64 = 10;
pub const RETRY_INTERVAL_SECS: u64 = 5;
pub const GCODE_CHECK_INTERVAL_MILLIS: u64 = 500;
pub const GCODE_WAIT_ATTEMPTS: u32 = 40;
pub const PRINT_OUTPUT_MAX_LINES: usize = 1000;
pub const SCULPT_LOAD_DELAY_MILLIS: u64 = 100;

pub const SUI_DECIMALS: f64 = 1_000_000_000.0;
pub const MESSAGE_AREA_MARGIN: u16 = 4;

pub struct NetworkPackageIds {
    pub eureka_package_id: &'static str,
    pub eureka_printer_registry_id: &'static str,
    #[allow(dead_code)]
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
        eureka_package_id: "",
        eureka_printer_registry_id: "",
        bottega_package_id: "",
    },
];

/// gRPC (HTTP/2 + TLS) endpoints for `sui_rpc::Client` (same hosts as public full nodes).
pub const NETWORKS: [(&str, &str); 3] = [
    ("devnet", sui_rpc::Client::DEVNET_FULLNODE),
    ("testnet", sui_rpc::Client::TESTNET_FULLNODE),
    ("mainnet", sui_rpc::Client::MAINNET_FULLNODE),
];