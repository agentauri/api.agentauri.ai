//! Action worker implementations
//!
//! Each worker type handles a specific action type (Telegram, REST, MCP).

pub mod telegram_worker;

pub use telegram_worker::TelegramWorker;
