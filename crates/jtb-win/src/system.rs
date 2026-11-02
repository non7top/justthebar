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

/// The default speakers' volume in percent and whether they are muted, or why it cannot be read.
pub fn volume() -> Result<(u8, bool), String> {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    let step = |what: &'static str| move |e: windows::core::Error| format!("{what}: {e}");
    unsafe {
        // Already initialized on this thread is fine; a different model is too.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(step("enumerator"))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(step("default device"))?;
        let volume: IAudioEndpointVolume = device
            .Activate(CLSCTX_ALL, None)
            .map_err(step("endpoint volume"))?;
        let level = volume.GetMasterVolumeLevelScalar().map_err(step("level"))?;
        let muted = volume.GetMute().map_err(step("mute"))?.as_bool();
        Ok(((level * 100.0).round().clamp(0.0, 100.0) as u8, muted))
    }
}

/// The current Wi-Fi connection, else a wired one, else offline.
pub fn network() -> Network {
    if let Some((name, quality)) = wifi() {
        return Network::Wifi { name, quality };
    }
    let adapters = adapters();
    let online = |a: &&Adapter| a.up && a.gateway;
    if let Some(a) = adapters.iter().filter(online).find(|a| a.kind == WIFI) {
        return Network::Wifi {
            name: a.name.clone(),
            quality: 100,
        };
    }
    if adapters
        .iter()
        .filter(online)
        .any(|a| a.kind == ETHERNET && !a.is_virtual())
    {
        return Network::Ethernet;
    }
    Network::Offline
}

/// One line per network adapter, for the log when the widget finds no connection.
pub fn network_report() -> String {
    adapters()
        .iter()
        .map(|a| {
            format!(
                "[type {} up {} gateway {} {} / {}]",
                a.kind, a.up, a.gateway, a.name, a.description
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const ETHERNET: u32 = 6;
const WIFI: u32 = 71;

struct Adapter {
    kind: u32,
    up: bool,
    gateway: bool,
    name: String,
    description: String,
}

impl Adapter {
    /// VPN and virtual switch adapters also have a gateway but are not the connection itself.
    fn is_virtual(&self) -> bool {
        let text = self.description.to_lowercase();
        [
            "tap", "wintun", "tun", "vpn", "virtual", "vmware", "hyper-v", "loopback",
        ]
        .iter()
        .any(|word| text.contains(word))
    }
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

fn adapters() -> Vec<Adapter> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::AF_UNSPEC;
    let mut size = 16 * 1024u32;
    for _ in 0..4 {
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        let status = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                None,
                Some(first),
                &mut size,
            )
        };
        const BUFFER_OVERFLOW: u32 = 111;
        if status == BUFFER_OVERFLOW {
            continue;
        }
        if status != 0 {
            return Vec::new();
        }
        let mut found = Vec::new();
        let mut adapter = first as *const IP_ADAPTER_ADDRESSES_LH;
        while !adapter.is_null() {
            let a = unsafe { &*adapter };
            let text = |p: windows::core::PWSTR| {
                if p.is_null() {
                    String::new()
                } else {
                    unsafe { p.to_string() }.unwrap_or_default()
                }
            };
            found.push(Adapter {
                kind: a.IfType,
                up: a.OperStatus == IfOperStatusUp,
                gateway: !a.FirstGatewayAddress.is_null(),
                name: text(a.FriendlyName),
                description: text(a.Description),
            });
            adapter = a.Next;
        }
        return found;
    }
    Vec::new()
}

/// Opens the Windows 11 quick settings panel the way Win+A does.
pub fn open_quick_settings() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_A, VK_LWIN,
    };
    let key = |code: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [
        key(VK_LWIN, KEYBD_EVENT_FLAGS(0)),
        key(VK_A, KEYBD_EVENT_FLAGS(0)),
        key(VK_A, KEYEVENTF_KEYUP),
        key(VK_LWIN, KEYEVENTF_KEYUP),
    ];
    unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
}

/// Shows the input language popup and switches to the next layout, the way Win+Space does; the
/// Windows key is held for a moment so the popup stays up.
pub fn open_input_switcher() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_LWIN, VK_SPACE,
    };
    fn send(code: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: code,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
    }
    send(VK_LWIN, KEYBD_EVENT_FLAGS(0));
    send(VK_SPACE, KEYBD_EVENT_FLAGS(0));
    send(VK_SPACE, KEYEVENTF_KEYUP);
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        send(VK_LWIN, KEYEVENTF_KEYUP);
    });
}
