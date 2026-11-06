//! Opt-in integration check. Briefly changes the real current-user LAN proxy,
//! then checks both orderly restore and recovery after terminating its owner.
//! Does not inspect windows, transmit credentials or change saved app settings.
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use multilink_platform::{proxy::ProxySettings, SystemProxyGuard};
    use std::os::windows::process::CommandExt;
    use std::{
        io::{BufRead, Write},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    if multilink_platform::handle_watchdog_args() {
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--owner") {
        let _guard = SystemProxyGuard::enable(17890)?;
        println!("OWNER_READY");
        std::io::stdout().flush()?;
        loop {
            std::thread::park();
        }
    }
    // A second invocation can compare a fingerprint without printing PAC URLs.
    multilink_platform::recover_system_proxy()?;
    let original = ProxySettings::read()?;
    let mut guard = SystemProxyGuard::enable(17890)?;
    assert!(
        ProxySettings::read()?.server == "http=127.0.0.1:17890;https=127.0.0.1:17890",
        "local proxy was not applied"
    );
    assert!(
        std::thread::spawn(|| SystemProxyGuard::enable(17890).is_err())
            .join()
            .unwrap(),
        "a second owner must be rejected"
    );
    assert!(SystemProxyGuard::enable(0).is_err());
    guard.restore()?;
    assert!(
        ProxySettings::read()? == original,
        "original proxy was not restored"
    );
    println!("Normal restore: passed");
    // Releasing the guard also wakes its watcher. Allow that helper to finish
    // before starting a new independent owner of the same named mutex.
    std::thread::sleep(Duration::from_millis(100));
    let mut owner = Command::new(std::env::current_exe()?)
        .arg("--owner")
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = owner.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let ready = rx.recv_timeout(Duration::from_secs(8));
    if !matches!(ready,Ok(ref s) if s.trim()=="OWNER_READY") {
        let _ = owner.kill();
        let _ = owner.wait();
        let _ = multilink_platform::recover_system_proxy();
        return Err("Proxy test owner did not become ready".into());
    }
    owner.kill()?;
    owner.wait()?;
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        if ProxySettings::read()? == original
            && !multilink_platform::data_dir()
                .join("system-proxy.dpapi")
                .exists()
        {
            break;
        }
        if Instant::now() >= until {
            multilink_platform::recover_system_proxy()?;
            return Err("Watchdog did not restore the proxy in time".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(!multilink_platform::data_dir()
        .join("system-proxy.dpapi")
        .exists());
    println!("Crash recovery via watchdog: passed");
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows only");
}
