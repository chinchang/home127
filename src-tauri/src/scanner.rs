use futures::future::join_all;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;
use tokio::net::TcpStream;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerInfo {
    port: u16,
    title: String,
    url: String,
    pid: Option<u32>,
    path: Option<String>,
    command: Option<String>,
    active: bool,
}

#[tauri::command]
pub async fn scan_servers(app: AppHandle) -> Vec<ServerInfo> {
    // Common development ports
    let ports = vec![
        3000, 3001, 3002, 3003, 3004, 3005, 8000, 8001, 8002, 8080, 8081, 5173, 5174, // Vite
        4200, // Angular
        4321, // Astro
        5000, 5001, // Flask/Python
        8888, // Jupyter
        1313, // Hugo
        4000, // Jekyll
        3333,
    ];

    let client = Client::builder()
        .timeout(Duration::from_millis(2000))
        .build()
        .unwrap_or_else(|_| Client::new());

    // 1. Scan for running servers
    let futures = ports.into_iter().map(|port| {
        let client = client.clone();
        async move {
            if is_port_open(port).await {
                if let Some(mut info) = check_http_server(port, &client).await {
                    info.pid = get_pid_for_port(port);
                    if let Some(pid) = info.pid {
                        info.path = get_cwd_for_pid(pid);
                        info.command = get_cmdline_from_pid(pid as i32);
                    }
                    println!("[scanner] Port {} -> Command: {:?}", port, info.command);
                    info.active = true;
                    return Some(info);
                }
            }
            None
        }
    });

    let running_servers: Vec<ServerInfo> = join_all(futures)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect();

    // 2. Load persisted servers
    let mut persisted_servers = load_servers(&app).unwrap_or_default();

    // 3. Merge logic
    // We use CWD (path) as the unique identifier.
    // If a running server has a path, we update the persisted entry or add it.
    // If a persisted server is not found in running_servers, we mark it as inactive.

    // Create a map of running servers by path for easy lookup
    let mut running_map: HashMap<String, ServerInfo> = HashMap::new();
    for server in running_servers {
        if let Some(path) = &server.path {
            running_map.insert(path.clone(), server);
        }
    }

    // Update persisted servers with running info, or mark as inactive
    for server in persisted_servers.iter_mut() {
        if let Some(path) = &server.path {
            if let Some(running) = running_map.remove(path) {
                // Server is running, update info
                *server = running;
            } else {
                // Server is not running, mark as inactive
                server.active = false;
                server.pid = None;
                // Keep port, title, url, command, path as is
            }
        }
    }

    // Add any new running servers that weren't in persisted list
    for (_, server) in running_map {
        persisted_servers.push(server);
    }

    // 4. Save updated list
    if let Err(e) = save_servers(&app, &persisted_servers) {
        eprintln!("Failed to save servers: {}", e);
    }

    persisted_servers
}

fn get_servers_file_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|p| {
        if !p.exists() {
            let _ = fs::create_dir_all(&p);
        }
        p.join("servers.json")
    })
}

fn load_servers(app: &AppHandle) -> Option<Vec<ServerInfo>> {
    let path = get_servers_file_path(app)?;
    if path.exists() {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn save_servers(app: &AppHandle, servers: &Vec<ServerInfo>) -> std::io::Result<()> {
    if let Some(path) = get_servers_file_path(app) {
        let content = serde_json::to_string_pretty(servers)?;
        fs::write(path, content)?;
    }
    Ok(())
}

async fn is_port_open(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return TcpStream::connect(addr).await.is_ok();
        }
    }
    false
}

async fn check_http_server(port: u16, client: &Client) -> Option<ServerInfo> {
    let url = format!("http://localhost:{}", port);
    match client.get(&url).send().await {
        Ok(resp) => {
            let headers = resp.headers().clone();
            let content_type = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.contains("text/html") {
                return None;
            }

            let body = resp.text().await.unwrap_or_default();
            let document = Html::parse_document(&body);
            let selector = Selector::parse("title").unwrap();
            let title = document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_else(|| "Unknown Server".to_string());

            Some(ServerInfo {
                port,
                title: title.trim().to_string(),
                url,
                pid: None,
                path: None,
                command: None,
                active: true,
            })
        }
        Err(_) => None,
    }
}

fn get_pid_for_port(port: u16) -> Option<u32> {
    let output = Command::new("lsof")
        .arg("-i")
        .arg(format!(":{}", port))
        .arg("-sTCP:LISTEN")
        .arg("-t")
        .output()
        .ok()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout);
        // Take the first line if multiple PIDs are returned
        pid_str.lines().next().and_then(|s| s.trim().parse().ok())
    } else {
        None
    }
}

fn get_cwd_for_pid(pid: u32) -> Option<String> {
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
        for line in output_str.lines() {
            if line.starts_with('n') {
                return Some(line[1..].to_string());
            }
        }
    }
    None
}

// Replaces get_command_for_pid with KERN_PROCARGS2 logic
#[cfg(target_os = "macos")]
fn get_cmdline_from_pid(pid: i32) -> Option<String> {
    use libc::{c_int, c_void, sysctl, CTL_KERN, KERN_PROCARGS2};
    use std::ptr;

    let mut mib: [c_int; 3] = [CTL_KERN, KERN_PROCARGS2, pid];
    let mut args_buffer: Vec<u8> = vec![0; 16384]; // 16KB buffer to handle large env vars
    let mut len: usize = args_buffer.len();

    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            3,
            args_buffer.as_mut_ptr() as *mut c_void,
            &mut len,
            ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
    }

    // Buffer format:
    // [argc: c_int] [exec_path: string] \0 [padding: \0...] [arg0] \0 [arg1] \0 ...

    if len < std::mem::size_of::<c_int>() {
        return None;
    }

    let argc = unsafe { *(args_buffer.as_ptr() as *const c_int) };
    let mut current_pos = std::mem::size_of::<c_int>();

    // Skip executable path (starts at position 4)
    while current_pos < len && args_buffer[current_pos] != 0 {
        current_pos += 1;
    }

    if current_pos >= len {
        return None;
    }

    // Skip trailing nulls (padding)
    while current_pos < len && args_buffer[current_pos] == 0 {
        current_pos += 1;
    }

    if current_pos >= len {
        return None;
    }

    let mut args: Vec<String> = Vec::new();
    let mut arg_count = 0;

    while current_pos < len && arg_count < argc {
        let start = current_pos;
        while current_pos < len && args_buffer[current_pos] != 0 {
            current_pos += 1;
        }

        let arg_slice = &args_buffer[start..current_pos];
        let s = String::from_utf8_lossy(arg_slice).to_string();

        args.push(s);

        arg_count += 1;
        current_pos += 1; // skip the null separator
    }

    if args.is_empty() {
        None
    } else {
        Some(args.join(" "))
    }
}

#[cfg(not(target_os = "macos"))]
fn get_cmdline_from_pid(pid: i32) -> Option<String> {
    // Fallback for non-macOS (though we only target macOS mainly)
    let output = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("command=")
        .output()
        .ok()?;

    if output.status.success() {
        let cmd = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !cmd.is_empty() {
            return Some(cmd);
        }
    }
    None
}
