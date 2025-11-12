/// Devnet
pub const EUREKA_DEVNET_PACKAGE_ID: &str = "0xa73e8ca153ab4f98b212ac1ee21d69a9d2c7bfacf9ef0d63326bfe72f47727ba";
pub const EUREKA_DEVNET_PRINTER_REGISTRY_ID: &str = "0xbd50616301535dcb649ddfc628a7163527def73a97cb387b09c5f4352285ca86";
pub const SCULPT_DEVNET_PACKAGE_ID: &str = "0x2571c1e364b5647e1ee17b43f9f289e5c64ce3a0c38f6f9441a3f331e0083efa";

/// Testnet - Updated with simplified seal_approve (PrintJob-based auth)
pub const WALRUS_COIN_TYPE: &str = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL";
pub const EUREKA_TESTNET_PACKAGE_ID: &str = "0x4e43c7642828f9d8c410a47d7ed80b3df7711e49662c4704549dc05b23076bec";
pub const EUREKA_TESTNET_PRINTER_REGISTRY_ID: &str = "0xc368483b3bb2d6695d44f4e53e75a82cd5db36e32c59298f56452945eb46e302";
pub const SCULPT_TESTNET_PACKAGE_ID: &str = "0xc1814c4cbd4c23f306e886c7f8aace3ce1635d0a6e896b3bf35835139945d693";
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

pub const NETWORKS: [(&str, &str); 3] = [
    ("devnet", "https://fullnode.devnet.sui.io:443"),
    ("testnet", "https://fullnode.testnet.sui.io:443"),
    ("mainnet", "https://fullnode.mainnet.sui.io:443"),
];