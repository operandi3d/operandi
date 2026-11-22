//! Operandi's Model Context Protocol bridge. See `docs/MCP_BRIDGE.md`.
//!
//! `bridge` maps MCP tool calls onto `operandi_core::Command`; `protocol`
//! handles JSON-RPC 2.0 framing and the MCP lifecycle. Neither touches the
//! GUI. The sidecar binary hosts one headless `Bridge` over stdio; `server`
//! lets the running app answer the same messages over a local socket, and
//! `operandi-mcp --attach` forwards stdio to it.
pub mod bridge;
pub mod commands;
pub mod import;
pub mod primitives;
pub mod properties;
pub mod protocol;
#[cfg(unix)]
pub mod server;
pub mod shading;
pub mod slices;
pub use bridge::Bridge;
