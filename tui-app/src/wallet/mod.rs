// Wallet module for handling blockchain interactions

mod types;
mod client;
mod printer;
mod sculpt;
mod print_job;
mod utils;

// Re-export the main structures and functions
pub use types::{SculptItem, PrinterInfo};
pub use client::Wallet;