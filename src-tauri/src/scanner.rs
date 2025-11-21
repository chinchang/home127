use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::process::Command;

#[derive(Serialize)]
pub struct ServerInfo {
    port: u16,
    title: String,
    url: String,
    pid: Option<u32>,
    path: Option<String>,
}

#[tauri::command]
pub fn scan_servers() -> Vec<ServerInfo> {
    // Common development ports
    let ports = vec![
        3000, 3001, 3002, 3003, 3004, 3005,
        8000, 8001, 8002, 8080, 8081,
        5173, 5174, // Vite
        4200, // Angular
        4321, // Astro
        5000, 5001, // Flask/Python
        8888, // Jupyter
        1313, // Hugo
        4000, // Jekyll
    ];
    
    let mut servers = Vec::new();
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap_or_else(|_| Client::new());

    for port in ports {
        if is_port_open(port) {
            if let Some(mut info) = check_http_server(port, &client) {
                info.pid = get_pid_for_port(port);
                if let Some(pid) = info.pid {
                    info.path = get_cwd_for_pid(pid);
                }
                servers.push(info);
            }
        }
    }
    
    servers
}

fn is_port_open(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok();
        }
    }
    false
}

fn check_http_server(port: u16, client: &Client) -> Option<ServerInfo> {
    let url = format!("http://localhost:{}", port);
    match client.get(&url).send() {
        Ok(resp) => {
            // Basic check: if it returns 200 OK (or even 404/403, it's a server)
            // But we want HTML servers.
            let headers = resp.headers().clone();
            let content_type = headers.get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.contains("text/html") {
                return None;
            }

            let body = resp.text().unwrap_or_default();
            let document = Html::parse_document(&body);
            let selector = Selector::parse("title").unwrap();
            let title = document.select(&selector).next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_else(|| "Unknown Server".to_string());
                
            Some(ServerInfo {
                port,
                title: title.trim().to_string(),
                url,
                pid: None, // Will be populated later
                path: None, // Will be populated later
            })
        },
        Err(_) => None,
    }
}

fn get_pid_for_port(port: u16) -> Option<u32> {
    let output = Command::new("lsof")
        .arg("-i")
        .arg(format!("tcp:{}", port))
        .arg("-sTCP:LISTEN")
        .arg("-t")
        .output()
        .ok()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout);
        // Take the first line if multiple (though usually one for a specific port listener)
        pid_str.lines().next().and_then(|s| s.trim().parse().ok())
    } else {
        None
    }
}

fn get_cwd_for_pid(pid: u32) -> Option<String> {
    println!("pid: {}", pid);
    let output = Command::new("lsof")
        .arg("-a")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-d")
        .arg("cwd")
        .arg("-F")
        .arg("n")
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Output format is like:
        // p12345
        // n/path/to/cwd
        for line in output_str.lines() {
            if line.starts_with('n') {
                return Some(line[1..].to_string());
            }
        }
    }
    None
}
