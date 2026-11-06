//! Per-user LAN proxy via WinINet. No elevation and no machine-wide WinHTTP changes.
//! The session mutex serializes owners. A separate process restores a durable
//! DPAPI journal after the owning worker/process dies (abandoned mutex).
use serde::{Deserialize, Serialize};
use std::{
    ffi::c_void,
    io::{self, BufRead, Write},
    marker::PhantomData,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Stdio},
    ptr::null,
    rc::Rc,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GlobalFree, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Networking::WinInet::*,
    Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
    System::{JobObjects::*, Threading::*},
};

const MUTEX: &str = "Local\\Everty.MultiLink.SystemProxy";
const BYPASS: &str = "localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Mutex ownership belongs to the OS thread: deliberately !Send and !Sync.
struct ProxyLock {
    handle: Handle,
    owned: bool,
    _thread: PhantomData<Rc<()>>,
}
impl ProxyLock {
    fn open() -> io::Result<Self> {
        let name = wide(MUTEX);
        let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            handle: Handle(handle),
            owned: false,
            _thread: PhantomData,
        })
    }
    fn acquire(&mut self, timeout: u32) -> io::Result<()> {
        match unsafe { WaitForSingleObject(self.handle.0, timeout) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {
                self.owned = true;
                Ok(())
            }
            WAIT_TIMEOUT => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Системный прокси уже используется другим экземпляром MultiLink",
            )),
            _ => Err(io::Error::last_os_error()),
        }
    }
}
impl Drop for ProxyLock {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                ReleaseMutex(self.handle.0);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxySettings {
    pub flags: u32,
    pub server: String,
    pub bypass: String,
    pub auto_config_url: String,
}
impl ProxySettings {
    pub fn read() -> io::Result<Self> {
        let ids = [
            INTERNET_PER_CONN_FLAGS_UI,
            INTERNET_PER_CONN_PROXY_SERVER,
            INTERNET_PER_CONN_PROXY_BYPASS,
            INTERNET_PER_CONN_AUTOCONFIG_URL,
        ];
        let mut options = ids.map(|id| INTERNET_PER_CONN_OPTIONW {
            dwOption: id,
            ..Default::default()
        });
        let mut list = INTERNET_PER_CONN_OPTION_LISTW {
            dwSize: std::mem::size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            dwOptionCount: options.len() as u32,
            pOptions: options.as_mut_ptr(),
            ..Default::default()
        };
        let mut size = list.dwSize;
        // WinINet allocates queried strings with GlobalAlloc; free every one,
        // including partially filled results from an unsuccessful query.
        let ok = unsafe {
            InternetQueryOptionW(
                null(),
                INTERNET_OPTION_PER_CONNECTION_OPTION,
                (&mut list as *mut INTERNET_PER_CONN_OPTION_LISTW).cast(),
                &mut size,
            )
        };
        let error = io::Error::last_os_error();
        let mut strings = Vec::new();
        for option in &options[1..] {
            let pointer = unsafe { option.Value.pszValue };
            let value = if pointer.is_null() {
                String::new()
            } else {
                unsafe {
                    let mut len = 0;
                    while *pointer.add(len) != 0 {
                        len += 1;
                    }
                    let value = String::from_utf16_lossy(std::slice::from_raw_parts(pointer, len));
                    GlobalFree(pointer.cast());
                    value
                }
            };
            strings.push(value);
        }
        if ok == 0 {
            return Err(error);
        }
        Ok(Self {
            flags: unsafe { options[0].Value.dwValue },
            server: strings.remove(0),
            bypass: strings.remove(0),
            auto_config_url: strings.remove(0),
        })
    }
    fn local(port: u16, original: &Self) -> Self {
        Self {
            flags: PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY,
            server: format!("127.0.0.1:{port}"),
            bypass: BYPASS.into(),
            auto_config_url: original.auto_config_url.clone(),
        }
    }
    fn write(&self) -> io::Result<()> {
        let mut server = wide(&self.server);
        let mut bypass = wide(&self.bypass);
        let mut pac = wide(&self.auto_config_url);
        let mut options = [
            INTERNET_PER_CONN_OPTIONW {
                dwOption: INTERNET_PER_CONN_FLAGS,
                Value: INTERNET_PER_CONN_OPTIONW_0 {
                    dwValue: self.flags,
                },
            },
            INTERNET_PER_CONN_OPTIONW {
                dwOption: INTERNET_PER_CONN_PROXY_SERVER,
                Value: INTERNET_PER_CONN_OPTIONW_0 {
                    pszValue: server.as_mut_ptr(),
                },
            },
            INTERNET_PER_CONN_OPTIONW {
                dwOption: INTERNET_PER_CONN_PROXY_BYPASS,
                Value: INTERNET_PER_CONN_OPTIONW_0 {
                    pszValue: bypass.as_mut_ptr(),
                },
            },
            INTERNET_PER_CONN_OPTIONW {
                dwOption: INTERNET_PER_CONN_AUTOCONFIG_URL,
                Value: INTERNET_PER_CONN_OPTIONW_0 {
                    pszValue: pac.as_mut_ptr(),
                },
            },
        ];
        let mut list = INTERNET_PER_CONN_OPTION_LISTW {
            dwSize: std::mem::size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            dwOptionCount: options.len() as u32,
            pOptions: options.as_mut_ptr(),
            ..Default::default()
        };
        // All pointed-to UTF-16 buffers and the option array outlive this call.
        unsafe {
            check(InternetSetOptionW(
                null(),
                INTERNET_OPTION_PER_CONNECTION_OPTION,
                (&mut list as *mut INTERNET_PER_CONN_OPTION_LISTW).cast(),
                list.dwSize,
            ))?;
            check(InternetSetOptionW(
                null(),
                INTERNET_OPTION_SETTINGS_CHANGED,
                null(),
                0,
            ))?;
            check(InternetSetOptionW(
                null(),
                INTERNET_OPTION_REFRESH,
                null(),
                0,
            ))?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    token: String,
    original: ProxySettings,
    applied: ProxySettings,
    #[serde(default)]
    committed: bool,
}
fn journal_path() -> PathBuf {
    super::data_dir().join("system-proxy.dpapi")
}
fn read_journal() -> io::Result<Option<Journal>> {
    match std::fs::read(journal_path()) {
        Ok(data) => serde_json::from_slice(&super::crypt(&data, false)?)
            .map(Some)
            .map_err(io::Error::other),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
fn persist(journal: &Journal) -> io::Result<()> {
    let bytes = super::crypt(
        &serde_json::to_vec(journal).map_err(io::Error::other)?,
        true,
    )?;
    std::fs::create_dir_all(super::data_dir())?;
    // A stale journal is recovered before this call. Never overwrite it.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(journal_path())?;
    let result = file.write_all(&bytes).and_then(|_| file.sync_all());
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(journal_path());
    }
    result
}
fn replace_journal(journal: &Journal) -> io::Result<()> {
    let bytes = super::crypt(
        &serde_json::to_vec(journal).map_err(io::Error::other)?,
        true,
    )?;
    let temp = super::data_dir().join(format!("proxy-{}.next", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    let src = wide(&temp.to_string_lossy());
    let dst = wide(&journal_path().to_string_lossy());
    let result = check(unsafe {
        MoveFileExW(
            src.as_ptr(),
            dst.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    });
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
fn should_restore(current: &ProxySettings, journal: &Journal) -> bool {
    !journal.committed || current == &journal.applied
}
fn restore_journal(journal: &Journal) -> io::Result<()> {
    let current = ProxySettings::read()?;
    // Do not undo a later manual change made by the user or another VPN.
    if should_restore(&current, journal) {
        // A crash in either setting or restoring options is recoverable, even
        // if Windows applied only some of the options before the interruption.
        let mut pending = journal.clone();
        pending.committed = false;
        replace_journal(&pending)?;
        journal.original.write()?;
        if ProxySettings::read()? != journal.original {
            return Err(io::Error::other(
                "Windows не восстановила исходные настройки прокси",
            ));
        }
    }
    match std::fs::remove_file(journal_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn recover_system_proxy() -> io::Result<bool> {
    let mut lock = ProxyLock::open()?;
    match lock.acquire(0) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
        Err(e) => return Err(e),
    }
    if let Some(journal) = read_journal()? {
        restore_journal(&journal)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub struct SystemProxyGuard {
    lock: Option<ProxyLock>,
    journal: Journal,
    _watchdog: Child,
    active: bool,
}
impl SystemProxyGuard {
    pub fn enable(port: u16) -> io::Result<Self> {
        if port == 0 {
            return Err(io::Error::other("Некорректный порт прокси"));
        }
        let mut lock = ProxyLock::open()?;
        lock.acquire(0)?;
        if let Some(journal) = read_journal()? {
            restore_journal(&journal)?;
        }
        let original = ProxySettings::read()?;
        let applied = ProxySettings::local(port, &original);
        let journal = Journal {
            token: uuid::Uuid::new_v4().to_string(),
            original,
            applied,
            committed: false,
        };
        persist(&journal)?;
        let watchdog = match spawn_watchdog(&journal.token) {
            Ok(child) => child,
            Err(e) => {
                let _ = std::fs::remove_file(journal_path());
                return Err(e);
            }
        };
        let mut guard = Self {
            lock: Some(lock),
            journal,
            _watchdog: watchdog,
            active: true,
        };
        if let Err(error) = guard.journal.applied.write().and_then(|_| {
            if ProxySettings::read()? == guard.journal.applied {
                Ok(())
            } else {
                Err(io::Error::other("Windows не применила системный прокси"))
            }
        }) {
            // The API can fail after writing some options. Roll back the
            // transaction unconditionally while we still own the mutex.
            if guard.journal.original.write().is_ok() {
                let _ = guard.restore();
            }
            return Err(error);
        }
        guard.journal.committed = true;
        if let Err(error) = replace_journal(&guard.journal) {
            guard.journal.committed = false;
            return Err(error);
        }
        Ok(guard)
    }
    pub fn restore(&mut self) -> io::Result<()> {
        if self.active {
            restore_journal(&self.journal)?;
            self.active = false;
            self.lock.take();
        }
        Ok(())
    }
}
impl Drop for SystemProxyGuard {
    fn drop(&mut self) {
        let _ = self.restore(); /* unlock even on failure: watchdog retries */
    }
}

fn spawn_watchdog(token: &str) -> io::Result<Child> {
    let mut child = Command::new(std::env::current_exe()?)
        .args(["--proxy-watchdog", token])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Нет канала watchdog"))?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = io::BufReader::new(stdout)
            .read_line(&mut line)
            .map(|_| line);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(line)) if line.trim() == "MULTILINK_PROXY_READY" => Ok(child),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            Err(io::Error::other(
                "Не удалось запустить восстановление системного прокси",
            ))
        }
    }
}

fn watchdog(token: &str) -> io::Result<()> {
    let mut lock = ProxyLock::open()?;
    println!("MULTILINK_PROXY_READY");
    io::stdout().flush()?;
    lock.acquire(INFINITE)?;
    for _ in 0..5 {
        match read_journal()? {
            Some(journal) if journal.token == token => {
                if restore_journal(&journal).is_ok() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            _ => return Ok(()),
        }
    }
    Err(io::Error::other(
        "Настройки прокси требуют восстановления при следующем запуске",
    ))
}
/// Call before initializing the UI or background workers in every host executable.
pub fn handle_watchdog_args() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--proxy-watchdog") {
        return false;
    }
    let result = args
        .get(2)
        .ok_or_else(|| io::Error::other("Нет токена watchdog"))
        .and_then(|token| watchdog(token));
    if result.is_err() {
        std::process::exit(1);
    }
    true
}

/// Closing the application process must also close its owned Mihomo process.
/// This prevents an orphaned core holding the local ports after a crash.
pub struct ChildJob(Handle);
impl ChildJob {
    pub fn attach(child: &Child) -> io::Result<Self> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Self(Handle(handle));
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            check(SetInformationJobObject(
                job.0 .0,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                std::mem::size_of_val(&info) as u32,
            ))?;
            check(AssignProcessToJobObject(
                job.0 .0,
                child.as_raw_handle().cast(),
            ))?;
        }
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn original() -> ProxySettings {
        ProxySettings {
            flags: PROXY_TYPE_DIRECT | PROXY_TYPE_AUTO_PROXY_URL | PROXY_TYPE_AUTO_DETECT,
            server: "http=old.example:8080".into(),
            bypass: "*.internal;<local>".into(),
            auto_config_url: "https://pac.example/config.pac".into(),
        }
    }
    #[test]
    fn local_proxy_disables_pac_detection_and_keeps_original_snapshot() {
        let before = original();
        let local = ProxySettings::local(17890, &before);
        assert_eq!(local.flags, PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY);
        assert_eq!(local.server, "127.0.0.1:17890");
        assert_eq!(
            before.flags,
            PROXY_TYPE_DIRECT | PROXY_TYPE_AUTO_PROXY_URL | PROXY_TYPE_AUTO_DETECT
        );
        let mut journal = Journal {
            token: "test".into(),
            original: before.clone(),
            applied: local.clone(),
            committed: true,
        };
        let roundtrip: Journal =
            serde_json::from_slice(&serde_json::to_vec(&journal).unwrap()).unwrap();
        assert_eq!(roundtrip.original, before);
        assert!(should_restore(&local, &journal));
        assert!(!should_restore(&before, &journal));
        let mut changed = local;
        changed.server = "http=another.example:8888".into();
        assert!(!should_restore(&changed, &journal));
        journal.committed = false;
        assert!(
            should_restore(&changed, &journal),
            "unfinished transaction must roll back partial writes"
        );
    }
    #[test]
    fn reads_current_user_proxy_without_modifying_it() {
        let state = ProxySettings::read().unwrap();
        assert!(state.flags > 0);
    }
}
