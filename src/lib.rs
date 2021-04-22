pub mod core;
pub mod ephemeral_message;
mod error;
pub mod events;
pub mod menu;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub use crate::core::RegisterRichInteractions;
pub use error::*;
