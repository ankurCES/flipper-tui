//! Flipper Zero domain operations: device info, storage, backup, firmware,
//! and update channel.
//!
//! Modules are added incrementally. v0.1 ships `device`, `exceptions`,
//! `protocol`, `storage`, `info`, and `backup`. `firmware` and
//! `updates` land in follow-up commits.

#![forbid(unsafe_code)]

pub mod backup;
pub mod device;
pub mod exceptions;
pub mod info;
pub mod protocol;
pub mod settings;
pub mod storage;
pub mod updates;

pub use backup::{request_backup, BackupState, BackupStatus};
pub use device::{BootMode, DeviceInfo, FlashInfo, HardwareInfo, RadioInfo};
pub use exceptions::FlipperError;
pub use info::{info, Info};
pub use protocol::{hello, parse_storage_list, read_file, stat, StorageEntry};
pub use settings::{parse_storage_info, storage_info, StorageInfo};
pub use storage::{mkdir, stat_file, write_file, FileStat, StatFlags};
pub use updates::{check, UpdateState, UpdateStatus};
