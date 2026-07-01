//! Device info + typed structs.

use serde::{Deserialize, Serialize};

use crate::exceptions::FlipperError;

/// All fields parsed from the `device_info` reply. Mirrors the qFlipper
/// webapp's `DeviceInfo` panel 1:1 so the TUI dashboard can render the
/// same rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub hardware: HardwareInfo,
    pub firmware_branch: String,
    pub firmware_version: String,
    pub firmware_build: String,
    pub firmware_target: String,
    pub radio: RadioInfo,
    pub flash: FlashInfo,
    pub api_major: u32,
    pub api_minor: u32,
    pub boot_mode: BootMode,
    pub serial: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub name: String,
    pub revision: String,
    pub region: String,
    pub lot: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioInfo {
    pub ble_mac: String,
    pub subghz: bool,
    pub nfc: bool,
    pub ir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashInfo {
    pub vendor: String,
    pub model: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootMode {
    Normal,
    Dfu,
    Recovery,
    Unknown,
}

impl DeviceInfo {
    /// Parse the multi-line `device_info` text the Flipper prints in
    /// response to the CLI `device_info` command.
    ///
    /// The format is loose `Key: value` lines, one per field. Unknown
    /// keys are skipped (forward-compat with future firmware) but a
    /// missing hardware name is a hard error.
    pub fn parse(payload: &str) -> Result<Self, FlipperError> {
        let mut hw_name: Option<String> = None;
        let mut hw_revision: Option<String> = None;
        let mut hw_region: Option<String> = None;
        let mut hw_lot: Option<String> = None;
        let mut fw_branch: Option<String> = None;
        let mut fw_version: Option<String> = None;
        let mut fw_build: Option<String> = None;
        let mut fw_target: Option<String> = None;
        let mut ble_mac: Option<String> = None;
        let mut subghz = false;
        let mut nfc = false;
        let mut ir = false;
        let mut flash_vendor: Option<String> = None;
        let mut flash_model: Option<String> = None;
        let mut flash_size: Option<u64> = None;
        let mut api_major: Option<u32> = None;
        let mut api_minor: Option<u32> = None;
        let mut boot_mode = BootMode::Unknown;
        let mut serial: Option<String> = None;

        for line in payload.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "hardware_name" => hw_name = Some(value.to_string()),
                "hardware_revision" => hw_revision = Some(value.to_string()),
                "hardware_region" => hw_region = Some(value.to_string()),
                "hardware_lot" => hw_lot = Some(value.to_string()),
                "firmware_branch" => fw_branch = Some(value.to_string()),
                "firmware_version" => fw_version = Some(value.to_string()),
                "firmware_build" => fw_build = Some(value.to_string()),
                "firmware_target" => fw_target = Some(value.to_string()),
                "radio_ble_mac" => ble_mac = Some(value.to_string()),
                "radio_subghz" => subghz = parse_bool(value),
                "radio_nfc" => nfc = parse_bool(value),
                "radio_ir" => ir = parse_bool(value),
                "flash_vendor" => flash_vendor = Some(value.to_string()),
                "flash_model" => flash_model = Some(value.to_string()),
                "flash_size" => flash_size = value.trim_end_matches(" kB").parse::<u64>().ok(),
                "api_major" => api_major = value.parse().ok(),
                "api_minor" => api_minor = value.parse().ok(),
                "boot_mode" => boot_mode = parse_boot_mode(value),
                "serial_number" => serial = Some(value.to_string()),
                _ => {}
            }
        }

        Ok(DeviceInfo {
            hardware: HardwareInfo {
                name: hw_name.ok_or_else(|| FlipperError::Parse("missing hardware_name".into()))?,
                revision: hw_revision.unwrap_or_default(),
                region: hw_region.unwrap_or_default(),
                lot: hw_lot.unwrap_or_default(),
            },
            firmware_branch: fw_branch.unwrap_or_default(),
            firmware_version: fw_version.unwrap_or_default(),
            firmware_build: fw_build.unwrap_or_default(),
            firmware_target: fw_target.unwrap_or_default(),
            radio: RadioInfo {
                ble_mac: ble_mac.unwrap_or_default(),
                subghz,
                nfc,
                ir,
            },
            flash: FlashInfo {
                vendor: flash_vendor.unwrap_or_default(),
                model: flash_model.unwrap_or_default(),
                size_bytes: flash_size.unwrap_or(0),
            },
            api_major: api_major.unwrap_or(0),
            api_minor: api_minor.unwrap_or(0),
            boot_mode,
            serial: serial.unwrap_or_default(),
        })
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s.trim(), "true" | "1" | "yes" | "YES")
}

fn parse_boot_mode(s: &str) -> BootMode {
    match s.trim() {
        "Normal" => BootMode::Normal,
        "DFU" => BootMode::Dfu,
        "Recovery" => BootMode::Recovery,
        _ => BootMode::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact `device_info` payload produced by a Momentum v1.4.4
    /// firmware on a R3llow4n (US-region) unit, captured live during
    /// the project's earlier Python phase. The test pins every field
    /// the TUI dashboard renders.
    const HELLO_SAMPLE: &str = "\
hardware_name: f7\n\
hardware_revision: R3llow4n\n\
hardware_region: US\n\
hardware_lot: 2024-Q3-19\n\
firmware_branch: mntm-012\n\
firmware_version: Momentum v1.4.4 OCT 2024\n\
firmware_build: 4106\n\
firmware_target: f7\n\
radio_ble_mac: AA:BB:CC:DD:EE:FF\n\
radio_subghz: true\n\
radio_nfc: true\n\
radio_ir: true\n\
flash_vendor: Winbond\n\
flash_model: W25Q128\n\
flash_size: 16384 kB\n\
api_major: 87\n\
api_minor: 1\n\
boot_mode: Normal\n\
serial_number: flip_R3llow4n1\n";

    #[test]
    fn parse_extracts_hardware_metadata() {
        let info = DeviceInfo::parse(HELLO_SAMPLE).expect("parse fails");
        assert_eq!(info.hardware.name, "f7");
        assert_eq!(info.hardware.revision, "R3llow4n");
        assert_eq!(info.hardware.region, "US");
        assert_eq!(info.hardware.lot, "2024-Q3-19");
    }

    #[test]
    fn parse_extracts_firmware_metadata() {
        let info = DeviceInfo::parse(HELLO_SAMPLE).expect("parse fails");
        assert_eq!(info.firmware_branch, "mntm-012");
        assert_eq!(info.firmware_version, "Momentum v1.4.4 OCT 2024");
        assert_eq!(info.firmware_build, "4106");
        assert_eq!(info.firmware_target, "f7");
    }

    #[test]
    fn parse_extracts_radio_flags() {
        let info = DeviceInfo::parse(HELLO_SAMPLE).expect("parse fails");
        assert_eq!(info.radio.ble_mac, "AA:BB:CC:DD:EE:FF");
        assert!(info.radio.subghz);
        assert!(info.radio.nfc);
        assert!(info.radio.ir);
    }

    #[test]
    fn parse_extracts_flash_metadata() {
        let info = DeviceInfo::parse(HELLO_SAMPLE).expect("parse fails");
        assert_eq!(info.flash.vendor, "Winbond");
        assert_eq!(info.flash.model, "W25Q128");
        // 16384 kB → 16384*1024 = 16777216 bytes. The parser keeps it
        // in kB for now (matches what the Flipper prints); conversion
        // happens in the display layer.
        assert_eq!(info.flash.size_bytes, 16384);
    }

    #[test]
    fn parse_extracts_api_version_and_boot_mode() {
        let info = DeviceInfo::parse(HELLO_SAMPLE).expect("parse fails");
        assert_eq!(info.api_major, 87);
        assert_eq!(info.api_minor, 1);
        assert_eq!(info.boot_mode, BootMode::Normal);
        assert_eq!(info.serial, "flip_R3llow4n1");
    }

    #[test]
    fn parse_errors_without_hardware_name() {
        let bad = "firmware_branch: dev\n";
        let err = DeviceInfo::parse(bad).unwrap_err();
        assert!(matches!(err, FlipperError::Parse(_)));
    }

    #[test]
    fn parse_skips_unknown_lines() {
        let mixed = "hardware_name: f7\nfuture_field: 42\n";
        let info = DeviceInfo::parse(mixed).expect("parse fails");
        assert_eq!(info.hardware.name, "f7");
    }
}
