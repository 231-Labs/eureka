/// Devnet
pub const EUREKA_DEVNET_PACKAGE_ID: &str = "0xa73e8ca153ab4f98b212ac1ee21d69a9d2c7bfacf9ef0d63326bfe72f47727ba";
pub const EUREKA_DEVNET_PRINTER_REGISTRY_ID: &str = "0xbd50616301535dcb649ddfc628a7163527def73a97cb387b09c5f4352285ca86";
pub const BOTTEGA_DEVNET_PACKAGE_ID: &str = "0x2571c1e364b5647e1ee17b43f9f289e5c64ce3a0c38f6f9441a3f331e0083efa";

/// Testnet
pub const WALRUS_COIN_TYPE: &str = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL";
pub const EUREKA_TESTNET_PACKAGE_ID: &str = "0x18815fa6daf0719bbab787c48588b7d4baaccc224e18c63446ad623c431555c7";
pub const EUREKA_TESTNET_PRINTER_REGISTRY_ID: &str = "0x19d08617c535d9a2299c40830d443f36bd31f2ea49be619a8448dcc89ecc9e97";
pub const BOTTEGA_TESTNET_PACKAGE_ID: &str = "0xd3406bbf426153af0957227f9ef864883c4c910cb037f7158d224a31759bdc26";
pub const AGGREGATOR_URL: &str = "https://walrus-agg-test.bucketprotocol.io";

pub struct NetworkPackageIds {
    pub eureka_package_id: &'static str,
    pub eureka_printer_registry_id: &'static str,
    pub bottega_package_id: &'static str,
}

pub const NETWORK_PACKAGE_IDS: [NetworkPackageIds; 3] = [
    NetworkPackageIds {
        eureka_package_id: EUREKA_DEVNET_PACKAGE_ID,
        eureka_printer_registry_id: EUREKA_DEVNET_PRINTER_REGISTRY_ID,
        bottega_package_id: BOTTEGA_DEVNET_PACKAGE_ID,
    },
    NetworkPackageIds {
        eureka_package_id: EUREKA_TESTNET_PACKAGE_ID,
        eureka_printer_registry_id: EUREKA_TESTNET_PRINTER_REGISTRY_ID,
        bottega_package_id: BOTTEGA_TESTNET_PACKAGE_ID,
    },
    NetworkPackageIds {
        eureka_package_id: "",  // 主網的 ID 待添加
        eureka_printer_registry_id: "",
        bottega_package_id: "",
    },
];

pub const NETWORKS: [(&str, &str); 3] = [
    ("devnet", "https://fullnode.devnet.sui.io:443"),
    ("testnet", "https://fullnode.testnet.sui.io:443"),
    ("mainnet", "https://fullnode.mainnet.sui.io:443"),
];

// ASCII藝術字EUREKA常量
pub const _EUREKA_ASCII_ART: &str = r#"
███████╗██╗   ██╗██████╗ ███████╗██╗  ██╗ █████╗ ██╗
██╔════╝██║   ██║██╔══██╗██╔════╝██║ ██╔╝██╔══██╗██║
█████╗  ██║   ██║██████╔╝█████╗  █████╔╝ ███████║██║
██╔══╝  ██║   ██║██╔══██╗██╔══╝  ██╔═██╗ ██╔══██║╚═╝
███████╗╚██████╔╝██║  ██║███████╗██║  ██╗██║  ██║██╗
╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝
"#;

// EUREKA動畫框架
pub const EUREKA_FRAMES: [&str; 3] = [
    r#"
███████╗██╗   ██╗██████╗ ███████╗██╗  ██╗ █████╗ 
██╔════╝██║   ██║██╔══██╗██╔════╝██║ ██╔╝██╔══██╗
█████╗  ██║   ██║██████╔╝█████╗  █████╔╝ ███████║
██╔══╝  ██║   ██║██╔══██╗██╔══╝  ██╔═██╗ ██╔══██║
███████╗╚██████╔╝██║  ██║███████╗██║  ██╗██║  ██║
╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝
"#,
    r#"
███████╗██╗   ██╗██████╗ ███████╗██╗  ██╗ █████╗ ▄▄
██╔════╝██║   ██║██╔══██╗██╔════╝██║ ██╔╝██╔══██╗  
█████╗  ██║   ██║██████╔╝█████╗  █████╔╝ ███████║  
██╔══╝  ██║   ██║██╔══██╗██╔══╝  ██╔═██╗ ██╔══██║  
███████╗╚██████╔╝██║  ██║███████╗██║  ██╗██║  ██║▀▀
╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝
"#,
    r#"
███████╗██╗   ██╗██████╗ ███████╗██╗  ██╗ █████╗ ▀▀
██╔════╝██║   ██║██╔══██╗██╔════╝██║ ██╔╝██╔══██╗  
█████╗  ██║   ██║██████╔╝█████╗  █████╔╝ ███████║  
██╔══╝  ██║   ██║██╔══██╗██╔══╝  ██╔═██╗ ██╔══██║  
███████╗╚██████╔╝██║  ██║███████╗██║  ██╗██║  ██║▄▄
╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝
"#,
];

// 添加構建在Sui上的標語
pub const BUILD_ON_SUI: &str = "Build on Sui";