//! Screen modules. v0.1 ships the read-only screens; interactive ones
//! (storage create/delete, firmware install) land in v0.2 once the
//! confirmation gates are wired up.

pub mod dashboard;
pub mod devices;
pub mod help;

pub use dashboard::Dashboard;
pub use devices::Devices;
pub use help::Help;
