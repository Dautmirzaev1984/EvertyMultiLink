use super::*;
use std::{
    io::Write,
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct ProxyStub {
    port: u16,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl ProxyStub {
    fn new(name: &'static str, latency: u64) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let failed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (fail, halt) = (failed.clone(), stop.clone());
        let task = thread::spawn(move || {
            while !halt.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    let fail = fail.clone();
                    thread::spawn(move || {
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        use std::io::BufRead;
                        let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                        let read_headers = |reader: &mut std::io::BufReader<
                            std::net::TcpStream,
                        >|
                         -> std::io::Result<String> {
                            let mut first = String::new();
                            reader.read_line(&mut first)?;
                            loop {
                                let mut line = String::new();
                                if reader.read_line(&mut line)? == 0 || line == "\r\n" {
                                    break;
                                }
                            }
                            Ok(first)
                        };
                        let first = match read_headers(&mut reader) {
                            Ok(line) => line,
                            _ => return,
                        };
                        if first.starts_with("CONNECT ") {
                            if stream
                                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                                .is_err()
                            {
                                return;
                            }
                            if read_headers(&mut reader).is_err() {
                                return;
                            }
                        }
                        thread::sleep(Duration::from_millis(latency));
                        let status = if fail.load(Ordering::Relaxed)
                            && first.contains("site-test.invalid")
                        {
                            "403 Forbidden"
                        } else {
                            "204 No Content"
                        };
                        let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Length: 0\r\nX-Test-Node: {name}\r\nConnection: close\r\n\r\n");
                    });
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });
        Self {
            port,
            failed,
            stop,
            thread: Some(task),
        }
    }
}
impl Drop for ProxyStub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.thread.take().unwrap().join();
    }
}
struct Core {
    child: Child,
    dir: PathBuf,
}
impl Drop for Core {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn smart_site_routes_choose_fast_proxy_and_fail_over_without_direct_route() {
    if !binary().is_file() {
        return;
    }
    let fast = ProxyStub::new("fast", 5);
    let slow = ProxyStub::new("slow", 230);
    let mut settings = Settings::default();
    settings.policy = Policy::Smart;
    settings.nodes = multilink_core::parse_subscription("vless://00000000-0000-4000-8000-000000000001@example.invalid:443#DE-fast\nvless://00000000-0000-4000-8000-000000000002@example.invalid:444#US-slow").unwrap();
    for (node, port) in settings.nodes.iter_mut().zip([fast.port, slow.port]) {
        node.proxy =
            serde_yaml::from_str(&format!("type: http\nserver: 127.0.0.1\nport: {port}")).unwrap();
    }
    settings.rules = vec!["DOMAIN-SUFFIX,site-test.invalid,MULTILINK".into()];
    let mut value: Value = serde_yaml::from_str(&config(&settings, "smart-test").unwrap()).unwrap();
    let (mixed, api) = (free_port(), free_port());
    value["mixed-port"] = json!(mixed);
    value["external-controller"] = json!(format!("127.0.0.1:{api}"));
    for group in value["proxy-groups"].as_array_mut().unwrap() {
        group["url"] = json!(if group["name"] == "MULTILINK" {
            "http://probe.invalid/"
        } else {
            "http://site-test.invalid/health"
        });
        group["interval"] = json!(1);
        group["lazy"] = json!(false);
        group["expected-status"] = json!("200-399/401/404");
    }
    let site_group = value["proxy-groups"][1]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    let dir = std::env::temp_dir().join(format!("everty-smart-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, serde_yaml::to_string(&value).unwrap()).unwrap();
    let mut command = Command::new(binary());
    command
        .arg("-d")
        .arg(&dir)
        .arg("-f")
        .arg(&path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let _core = Core {
        child: command.spawn().unwrap(),
        dir,
    };
    let controller = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let wait_for = |expected: &str| {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let current = controller
                .get(format!("http://127.0.0.1:{api}/proxies/{site_group}"))
                .bearer_auth("smart-test")
                .send()
                .ok()
                .and_then(|r| r.json::<Value>().ok());
            if current.as_ref().is_some_and(|v| v["now"] == expected) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "site group did not select {expected}: {current:?}"
            );
            thread::sleep(Duration::from_millis(100));
        }
    };
    let traffic = Client::builder()
        .no_proxy()
        .proxy(reqwest::Proxy::http(format!("http://127.0.0.1:{mixed}")).unwrap())
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    wait_for("DE-fast");
    let response = traffic.get("http://site-test.invalid/").send().unwrap();
    assert_eq!(
        response
            .headers()
            .get("X-Test-Node")
            .and_then(|h| h.to_str().ok()),
        Some("fast"),
        "status={} headers={:?}",
        response.status(),
        response.headers()
    );
    fast.failed.store(true, Ordering::Relaxed);
    wait_for("US-slow");
    let global: Value = controller
        .get(format!("http://127.0.0.1:{api}/proxies/MULTILINK"))
        .bearer_auth("smart-test")
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(
        global["now"], "DE-fast",
        "a site-specific failure must not switch all traffic"
    );
    let response = traffic.get("http://site-test.invalid/").send().unwrap();
    assert_eq!(
        response
            .headers()
            .get("X-Test-Node")
            .and_then(|h| h.to_str().ok()),
        Some("slow"),
        "status={} headers={:?}",
        response.status(),
        response.headers()
    );
}
