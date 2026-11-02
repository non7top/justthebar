//! Small reads of system state for the bar widgets.
use jtb_core::widget::Network;
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

/// The default speakers' volume in percent and whether they are muted.
pub fn volume() -> Option<(u8, bool)> {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let level = volume.GetMasterVolumeLevelScalar().ok()?;
        let muted = volume.GetMute().ok()?.as_bool();
        Some(((level * 100.0).round().clamp(0.0, 100.0) as u8, muted))
    }
}

/// The current Wi-Fi connection, else a wired one, else offline.
pub fn network() -> Network {
    if let Some((name, quality)) = wifi() {
        return Network::Wifi { name, quality };
    }
    if wired() {
        return Network::Ethernet;
    }
    Network::Offline
}

fn wifi() -> Option<(String, u8)> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::WiFi::{
        wlan_interface_state_connected, wlan_intf_opcode_current_connection, WlanCloseHandle,
        WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
        WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO_LIST,
    };
    unsafe {
        let mut version = 0;
        let mut client = HANDLE::default();
        if WlanOpenHandle(2, None, &mut version, &mut client) != 0 {
            return None;
        }
        let mut found = None;
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(client, None, &mut list) == 0 && !list.is_null() {
            let interfaces = std::slice::from_raw_parts(
                (*list).InterfaceInfo.as_ptr(),
                (*list).dwNumberOfItems as usize,
            );
            for interface in interfaces {
                let mut size = 0;
                let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
                let status = WlanQueryInterface(
                    client,
                    &interface.InterfaceGuid,
                    wlan_intf_opcode_current_connection,
                    None,
                    &mut size,
                    &mut data,
                    None,
                );
                if status == 0 && !data.is_null() {
                    let attributes = &*(data as *const WLAN_CONNECTION_ATTRIBUTES);
                    if attributes.isState == wlan_interface_state_connected {
                        let ssid = &attributes.wlanAssociationAttributes.dot11Ssid;
                        let name = String::from_utf8_lossy(
                            &ssid.ucSSID[..(ssid.uSSIDLength as usize).min(ssid.ucSSID.len())],
                        )
                        .into_owned();
                        let quality = attributes.wlanAssociationAttributes.wlanSignalQuality;
                        found = Some((name, quality.min(100) as u8));
                    }
                    WlanFreeMemory(data);
                }
                if found.is_some() {
                    break;
                }
            }
            WlanFreeMemory(list as *const _);
        }
        let _ = WlanCloseHandle(client, None);
        found
    }
}

/// True when a wired adapter is up and has a gateway.
fn wired() -> bool {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::AF_UNSPEC;
    const ETHERNET: u32 = 6;
    unsafe {
        let mut size = 0u32;
        let _ = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_GATEWAYS,
            None,
            None,
            &mut size,
        );
        if size == 0 {
            return false;
        }
        let mut buffer = vec![0u8; size as usize];
        let first = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        if GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_GATEWAYS,
            None,
            Some(first),
            &mut size,
        ) != 0
        {
            return false;
        }
        let mut adapter = first as *const IP_ADAPTER_ADDRESSES_LH;
        while !adapter.is_null() {
            let a = &*adapter;
            if a.IfType == ETHERNET
                && a.OperStatus == IfOperStatusUp
                && !a.FirstGatewayAddress.is_null()
            {
                return true;
            }
            adapter = a.Next;
        }
        false
    }
}
