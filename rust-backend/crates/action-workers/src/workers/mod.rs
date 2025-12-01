//! Action worker implementations
//!
//! Each worker type handles a specific action type (Telegram, REST, MCP).

pub mod rest_worker;
pub mod telegram_worker;

pub use rest_worker::RestWorker;
pub use telegram_worker::TelegramWorker;
