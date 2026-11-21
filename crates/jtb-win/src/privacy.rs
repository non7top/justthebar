use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    REG_QWORD, REG_VALUE_TYPE,
};

fn open(parent: HKEY, path: PCWSTR) -> Option<HKEY> {
    let mut key = HKEY::default();
    let status = unsafe { RegOpenKeyExW(parent, path, None, KEY_READ, &mut key) };
    (status == ERROR_SUCCESS).then_some(key)
}

fn read_qword(key: HKEY, name: PCWSTR) -> u64 {
    let mut value = 0u64;
    let mut size = size_of::<u64>() as u32;
    let mut kind = REG_VALUE_TYPE(0);
    let status = unsafe {
        RegQueryValueExW(
            key,
            name,
            None,
            Some(&mut kind),
            Some(&mut value as *mut u64 as *mut u8),
            Some(&mut size),
        )
    };
    if status == ERROR_SUCCESS && kind == REG_QWORD {
        value
    } else {
        0
    }
}

/// True when a subkey of `parent` is an app whose last microphone use has started and not stopped.
fn any_using(parent: HKEY) -> bool {
    let mut index = 0;
    loop {
        let mut name = [0u16; 512];
        let mut len = name.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                parent,
                index,
                Some(windows::core::PWSTR(name.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            return false;
        }
        index += 1;
        if let Some(app) = open(parent, PCWSTR(name.as_ptr())) {
            let start = read_qword(app, w!("LastUsedTimeStart"));
            let stop = read_qword(app, w!("LastUsedTimeStop"));
            unsafe {
                let _ = RegCloseKey(app);
            }
            if start > stop {
                return true;
            }
        }
    }
}

/// Whether any app is using the microphone right now, from Windows' privacy consent store; none
/// when the store cannot be read.
pub fn microphone_in_use() -> Option<bool> {
    let root = open(
        HKEY_CURRENT_USER,
        w!(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone"
        ),
    )?;
    let mut in_use = any_using(root);
    if let Some(desktop_apps) = open(root, w!("NonPackaged")) {
        in_use |= any_using(desktop_apps);
        unsafe {
            let _ = RegCloseKey(desktop_apps);
        }
    }
    unsafe {
        let _ = RegCloseKey(root);
    }
    Some(in_use)
}
