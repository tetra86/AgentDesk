//! Platform abstraction layer.
//!
//! Provides traits for OS-specific operations (binary lookup, process dump,
//! shell execution) so the rest of the codebase can be platform-agnostic.

pub mod binary_resolver;
mod dump_tool;
pub mod shell;

pub use binary_resolver::{resolve_binary, resolve_binary_with_login_shell, async_resolve_binary_with_login_shell};
pub use dump_tool::capture_process_dump;
pub use shell::{shell_command, async_shell_command};
