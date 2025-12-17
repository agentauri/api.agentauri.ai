//! Action worker implementations
//!
//! Each worker type handles a specific action type:
//! - Telegram: Send notifications via Telegram Bot API
//! - REST: Execute HTTP webhooks to external services
//!
//! Note: MCP worker is planned for Phase 5.

pub mod rest_worker;
pub mod telegram_worker;

pub use rest_worker::RestWorker;
pub use telegram_worker::TelegramWorker;
