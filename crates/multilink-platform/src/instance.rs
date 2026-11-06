//! One application per Windows session. The mutex is independent from the
//! proxy guard, and is intentionally bypassed by its crash-recovery helper.
use std::{io, ptr::null};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
    System::Threading::CreateMutexW,
    UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE},
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub struct Instance(HANDLE);
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn acquire_named(name: &str) -> io::Result<Option<Instance>> {
    let name = wide(name);
    let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(handle);
        };
        Ok(None)
    } else {
        Ok(Some(Instance(handle)))
    }
}
pub fn acquire() -> io::Result<Option<Instance>> {
    acquire_named("Local\\Everty.MultiLink.Application")
}
pub fn show_existing() -> bool {
    let title = wide("Everty MultiLink");
    let window = unsafe { FindWindowW(null(), title.as_ptr()) };
    if window.is_null() {
        return false;
    }
    unsafe {
        ShowWindow(window, SW_RESTORE);
        SetForegroundWindow(window);
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn session_mutex_allows_one_owner_and_releases_on_drop() {
        let name = format!("Local\\Everty.MultiLink.Test.{}", std::process::id());
        let first = super::acquire_named(&name)
            .unwrap()
            .expect("first instance");
        assert!(super::acquire_named(&name).unwrap().is_none());
        drop(first);
        assert!(super::acquire_named(&name).unwrap().is_some());
    }
}
