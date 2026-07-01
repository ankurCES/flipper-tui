//! USB-device enumeration for Flipper Zeros.

use std::time::Duration;

/// A detected Flipper endpoint on a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEndpoint {
    /// OS serial-port path (e.g. `/dev/tty.usbmodemflip_R3llow4n1`,
    /// `/dev/ttyACM0`).
    pub path: String,
    /// USB vendor id, always `0x0483` for `STMicroelectronics` (Flipper).
    pub vid: u16,
    /// USB product id; varies by boot mode:
    /// - `0x5740` — normal CDC mode (the mode qFlipper talks to)
    /// - `0xdf11` — DFU bootloader
    pub pid: u16,
    /// Best-effort serial number / friendly name from the OS.
    pub label: Option<String>,
}

/// Enumerate every serial port that looks like a Flipper.
///
/// Cheap on macOS/Linux — just walks `serialport::available_ports()` and
/// filters by `STMicro` VID. On Windows the CDC port may show up under a
/// `COMx` name; the same predicate applies.
pub fn detect_devices() -> Vec<DeviceEndpoint> {
    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };
    ports
        .into_iter()
        .filter_map(|p| match p.port_type {
            serialport::SerialPortType::UsbPort(usb) => Some(DeviceEndpoint {
                path: p.port_name,
                vid: usb.vid,
                pid: usb.pid,
                label: usb
                    .serial_number
                    .and_then(|s| if s.is_empty() { None } else { Some(s) }),
            }),
            _ => None,
        })
        .filter(|d| d.vid == 0x0483)
        .collect()
}

/// Wait up to `timeout` for a Flipper to appear. Polls every 250ms.
pub fn wait_for_device(timeout: Duration) -> Option<DeviceEndpoint> {
    let deadline = std::time::Instant::now() + timeout;
    let mut interval = Duration::from_millis(250);
    while std::time::Instant::now() < deadline {
        if let Some(dev) = detect_devices().into_iter().next() {
            return Some(dev);
        }
        std::thread::sleep(interval);
        interval = (interval * 2).min(Duration::from_secs(2));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_no_panic_when_no_flipper() {
        // We don't assert a specific count because the test runner may or
        // may not have a Flipper attached — just that it doesn't crash.
        let _ = detect_devices();
    }
}
