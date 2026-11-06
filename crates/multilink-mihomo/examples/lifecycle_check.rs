//! Opt-in test against the installed local core; uses no subscription secrets.
//! Run while the application is disconnected. No system proxy is modified.
use multilink_mihomo::{Engine, CONTROLLER, PORT};
use std::{
    io::{BufRead, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn start() -> Result<Engine, String> {
    let mut settings = multilink_core::Settings::default();
    settings.system_proxy = false;
    settings.nodes = multilink_core::parse_subscription(
        "vless://00000000-0000-4000-8000-000000000001@example.invalid:443#Lifecycle-check",
    )?;
    Engine::start(&settings)
}
fn ports_free() -> bool {
    [PORT, CONTROLLER]
        .iter()
        .all(|port| std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--owner") {
        let mut engine = start()?;
        engine.metrics()?;
        println!("CORE_READY");
        std::io::stdout().flush()?;
        loop {
            std::thread::park();
        }
    }
    let mut engine = start()?;
    engine.metrics()?;
    drop(engine);
    assert!(ports_free(), "normal shutdown did not release core ports");
    println!("Core startup, controller API and normal shutdown: passed");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
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
        if !matches!(rx.recv_timeout(Duration::from_secs(18)),Ok(ref line) if line.trim()=="CORE_READY")
        {
            let _ = owner.kill();
            let _ = owner.wait();
            return Err("Core owner did not become ready".into());
        }
        owner.kill()?;
        owner.wait()?;
        let until = Instant::now() + Duration::from_secs(5);
        while !ports_free() {
            if Instant::now() >= until {
                return Err("Core survived its owner process".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        println!("Job Object terminates core after owner crash: passed");
    }
    Ok(())
}
