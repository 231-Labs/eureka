// Refactored printer module
mod operations;
mod ui;
mod blockchain;
mod monitoring;
mod mock;

// Re-export mock module for easier access
pub use mock::{run_mock_print_script, MockPrintScriptResult};