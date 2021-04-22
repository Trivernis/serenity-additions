pub mod core;
pub mod ephemeral_message;
mod error;
pub mod event_handlers;
pub mod menu;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub use error::*;
