use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

pub struct LanSyncSession {
    pub url: String,
    pub pin: String,
    running: Arc<AtomicBool>,
}

impl LanSyncSession {
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

pub fn detect_lan_ip() -> String {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" {
                    return ip;
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn start_lan_sync(tx: Sender<String>) -> Result<LanSyncSession, String> {
    let pin = format!(
        "{:04}",
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 9000
            + 1000)
    );

    let (listener, port) = match TcpListener::bind("0.0.0.0:8899") {
        Ok(l) => (l, 8899),
        Err(_) => match TcpListener::bind("0.0.0.0:0") {
            Ok(l) => {
                let p = l.local_addr().map(|a| a.port()).unwrap_or(8899);
                (l, p)
            }
            Err(e) => return Err(format!("Не удалось привязать сетевой порт: {e}")),
        },
    };

    let ip = detect_lan_ip();
    let url = format!("http://{ip}:{port}?pin={pin}");
    let running = Arc::new(AtomicBool::new(true));
    let running_thread = running.clone();
    let pin_expected = pin.clone();

    thread::spawn(move || {
        let _ = listener.set_nonblocking(false);
        while running_thread.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let pin_check = pin_expected.clone();
                    let tx_cloned = tx.clone();
                    thread::spawn(move || {
                        handle_client(&mut stream, &pin_check, &tx_cloned);
                    });
                }
                Err(_) => {
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    });

    Ok(LanSyncSession { url, pin, running })
}

fn handle_client(stream: &mut TcpStream, pin: &str, tx: &Sender<String>) {
    let mut buf = [0u8; 8192];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");

    if first_line.starts_with("GET ") {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Everty MultiLink • Передача на ПК</title>
<style>
body {{ background: #0D1117; color: #E6EDF3; font-family: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif; display: flex; justify-content: center; align-items: center; min-height: 100vh; margin: 0; padding: 20px; }}
.card {{ background: #161C24; border: 1px solid #272F3B; border-radius: 16px; padding: 28px; width: 100%; max-width: 480px; box-shadow: 0 12px 36px rgba(0,0,0,0.6); }}
h2 {{ margin-top: 0; color: #6FE6B4; font-size: 22px; font-weight: 700; }}
p {{ color: #8B949E; font-size: 14px; line-height: 1.5; }}
label {{ display: block; font-size: 13px; font-weight: 600; color: #8B949E; margin-top: 16px; margin-bottom: 6px; }}
input, textarea {{ width: 100%; background: #0D1117; border: 1px solid #272F3B; border-radius: 8px; color: #FFF; padding: 12px; font-size: 14px; box-sizing: border-box; }}
input:focus, textarea:focus {{ outline: none; border-color: #6FE6B4; }}
button {{ width: 100%; margin-top: 20px; background: #6FE6B4; color: #0D1117; font-weight: 700; border: none; border-radius: 8px; padding: 14px; font-size: 16px; cursor: pointer; }}
button:active {{ opacity: 0.8; }}
</style>
</head>
<body>
<div class="card">
  <h2>Everty MultiLink • Синхронизация</h2>
  <p>Вставьте подписку или VLESS-ссылки для отправки на ваш компьютер:</p>
  <form method="POST" action="/api/import">
    <label>PIN-код подтверждения:</label>
    <input type="text" name="pin" value="{pin}" required>
    <label>Ссылка на подписку или VLESS:</label>
    <textarea name="config" rows="6" placeholder="https://... или vless://..." required></textarea>
    <button type="submit">Отправить на ПК</button>
  </form>
</div>
</body>
</html>"#
        );
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(),
            html
        );
        let _ = stream.write_all(resp.as_bytes());
    } else if first_line.starts_with("POST ") {
        let mut full = buf[..n].to_vec();
        let mut content_len = 0;
        for line in req.lines() {
            if line.to_ascii_lowercase().starts_with("content-length:") {
                content_len = line[15..].trim().parse::<usize>().unwrap_or(0);
            }
        }
        let header_end = req.find("\r\n\r\n").map(|p| p + 4).unwrap_or(n);
        let current_body_len = full.len().saturating_sub(header_end);
        let mut remaining = content_len.saturating_sub(current_body_len);
        while remaining > 0 {
            let mut chunk = [0u8; 4096];
            let r = match stream.read(&mut chunk) {
                Ok(r) if r > 0 => r,
                _ => break,
            };
            full.extend_from_slice(&chunk[..r]);
            remaining = remaining.saturating_sub(r);
        }
        let body_bytes = &full[header_end..];
        let body_str = String::from_utf8_lossy(body_bytes);

        let mut req_pin = String::new();
        let mut config = String::new();

        if body_str.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                req_pin = val
                    .get("pin")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                config = val
                    .get("config")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        } else {
            for pair in body_str.split('&') {
                let mut kv = pair.splitn(2, '=');
                let k = kv.next().unwrap_or("");
                let v = kv.next().unwrap_or("");
                let decoded = url_decode(v);
                if k == "pin" {
                    req_pin = decoded;
                } else if k == "config" {
                    config = decoded;
                }
            }
        }

        if req_pin.trim() == pin.trim() && !config.trim().is_empty() {
            let _ = tx.send(config);
            let resp_body = r#"{"ok":true,"message":"Успешно получено"}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.len(),
                resp_body
            );
            let _ = stream.write_all(resp.as_bytes());
        } else {
            let resp_body = r#"{"ok":false,"error":"Неверный PIN-код"}"#;
            let resp = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.len(),
                resp_body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    }
}

fn url_decode(s: &str) -> String {
    let replaced = s.replace('+', " ");
    let mut result = Vec::new();
    let bytes = replaced.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                result.push(val);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}
