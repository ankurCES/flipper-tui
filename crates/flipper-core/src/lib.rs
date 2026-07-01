//! Flipper Zero domain operations: device info, storage, backup, firmware,
//! and update channel.
//!
//! Modules are added incrementally. v0.1 ships `device`, `exceptions`,
//! and `protocol`. The remaining modules (`storage`, `backup`, `firmware`,
//! `updates`) land in follow-up commits.

#![forbid(unsafe_code)]

pub mod device;
pub mod exceptions;
pub mod protocol;
pub mod storage;

pub use device::{BootMode, DeviceInfo, FlashInfo, HardwareInfo, RadioInfo};
pub use exceptions::FlipperError;
pub use protocol::{hello, parse_storage_list, read_file, stat, StorageEntry};
pub use storage::{mkdir, stat_file, write_file, FileStat, StatFlags};
