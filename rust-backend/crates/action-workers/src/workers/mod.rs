//! Action worker implementations
//!
//! Each worker type handles a specific action type:
//! - Telegram: Send notifications via Telegram Bot API
//! - REST: Execute HTTP webhooks to external services
//! - MCP: Execute tool calls via Model Context Protocol

pub mod mcp_worker;
pub mod rest_worker;
pub mod telegram_worker;

pub use mcp_worker::McpWorker;
pub use rest_worker::RestWorker;
pub use telegram_worker::TelegramWorker;
