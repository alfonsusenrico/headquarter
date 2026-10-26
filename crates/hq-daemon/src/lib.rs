//! The Headquarter host: owns agent sessions and their PTYs, the session index, and
//! the sockets clients and hooks connect to. See `openspec/specs/session-host/`.

mod host;
mod log;
mod modes;
pub mod paths;
mod session;

pub use host::{HostError, HostOptions, run};
