//! Small reads of system state for the bar widgets.
use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME2};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Battery charge in percent and whether it is charging; none on a machine without a battery.
pub fn battery() -> Option<(u8, bool)> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    const NO_BATTERY: u8 = 128;
    const UNKNOWN: u8 = 255;
    if status.BatteryFlag == NO_BATTERY || status.BatteryFlag == UNKNOWN {
        return None;
    }
    let percent = status.BatteryLifePercent.min(100);
    let charging = status.BatteryFlag & 8 != 0 || status.ACLineStatus == 1 && percent < 100;
    Some((percent, charging))
}

/// The three letter ISO 639-2 code (`eng`, `rus`) of the keyboard layout of the foreground window.
pub fn input_language() -> Option<String> {
    let thread = unsafe { GetWindowThreadProcessId(GetForegroundWindow(), None) };
    let layout = unsafe { GetKeyboardLayout(thread) };
    let language_id = (layout.0 as usize & 0xFFFF) as u32;
    let mut buffer = [0u16; 16];
    let len = unsafe { GetLocaleInfoW(language_id, LOCALE_SISO639LANGNAME2, Some(&mut buffer)) };
    (len > 1).then(|| String::from_utf16_lossy(&buffer[..len as usize - 1]))
}
