use futures::future::join_all;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerInfo {
    port: u16,
    title: String,
    url: String,
    pid: Option<u32>,
    path: Option<String>,
    command: Option<String>,
    active: bool,
    #[serde(default)]
    custom_name: Option<String>,
}

#[tauri::command]
pub async fn scan_servers(app: AppHandle) -> Vec<ServerInfo> {
    // Dynamically discover all listening TCP ports
    let ports = discover_listening_ports();
    println!("[scanner] Discovered {} candidate ports: {:?}", ports.len(), ports);

    let client = Client::builder()
        .timeout(Duration::from_millis(2000))
        .build()
        .unwrap_or_else(|_| Client::new());

    // 1. Scan for running servers
    let futures = ports.into_iter().map(|port| {
        let client = client.clone();
        async move {
            if let Some(mut info) = check_http_server(port, &client).await {
                info.pid = get_pid_for_port(port);
                if let Some(pid) = info.pid {
                    info.path = get_cwd_for_pid(pid);
                    info.command = get_best_cmdline_for_pid(pid as i32);
                }
                println!("[scanner] Port {} -> Command: {:?}", port, info.command);
                info.active = true;
                return Some(info);
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
                // Server is running, update scanned fields but preserve user data
                server.port = running.port;
                server.title = running.title;
                server.url = running.url;
                server.pid = running.pid;
                server.path = running.path;
                server.command = running.command;
                server.active = running.active;
                // custom_name intentionally NOT overwritten
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

#[tauri::command]
pub async fn rename_server(
    app: AppHandle,
    path: String,
    custom_name: Option<String>,
) -> Result<Vec<ServerInfo>, String> {
    let mut servers = load_servers(&app).unwrap_or_default();

    let found = servers.iter_mut().find(|s| s.path.as_deref() == Some(&path));

    match found {
        Some(server) => {
            server.custom_name = custom_name
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty());

            save_servers(&app, &servers).map_err(|e| format!("Failed to save: {}", e))?;

            Ok(servers)
        }
        None => Err(format!("Server with path '{}' not found", path)),
    }
}

/// Discover all TCP ports currently in LISTEN state using lsof.
/// Filters out known system/database ports and ephemeral ports.
fn discover_listening_ports() -> Vec<u16> {
    let output = match Command::new("/usr/sbin/lsof")
        .arg("-iTCP")
        .arg("-sTCP:LISTEN")
        .arg("-P")
        .arg("-n")
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("[scanner] Failed to run lsof: {}", e);
            return Vec::new();
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = HashSet::new();

    for line in stdout.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // NAME is second-to-last field; last field is "(LISTEN)"
        // e.g. "TCP *:8080 (LISTEN)" → fields = [..., "TCP", "*:8080", "(LISTEN)"]
        if fields.len() < 2 {
            continue;
        }
        let name = fields[fields.len() - 2];

        // Parse port from NAME field: "*:8080", "127.0.0.1:3000", "[::1]:3000"
        if let Some(port_str) = name.rsplit(':').next() {
            if let Ok(port) = port_str.parse::<u16>() {
                ports.insert(port);
            }
        }
    }

    // System/infrastructure ports to skip (never dev web servers)
    let skip_ports: &[u16] = &[
        22,    // SSH
        53,    // DNS
        88,    // Kerberos
        443,   // HTTPS (system)
        445,   // SMB
        631,   // CUPS
        1900,  // SSDP/UPnP
        3306,  // MySQL
        3722,  // rapportd (Apple)
        5353,  // mDNS
        5432,  // PostgreSQL
        6379,  // Redis
        7000,  // AirPlay (ControlCenter)
        27017, // MongoDB
    ];

    for &p in skip_ports {
        ports.remove(&p);
    }

    // Skip ephemeral port range (OS-assigned, never intentional dev servers)
    ports.retain(|&p| p < 49152);

    let mut result: Vec<u16> = ports.into_iter().collect();
    result.sort();
    result
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
                custom_name: None,
            })
        }
        Err(_) => None,
    }
}

fn get_pid_for_port(port: u16) -> Option<u32> {
    let output = Command::new("/usr/sbin/lsof")
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
    let output = Command::new("/usr/sbin/lsof")
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

/// Get parent PID for a given PID
fn get_ppid(pid: u32) -> Option<u32> {
    let output = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("ppid=")
        .output()
        .ok()?;

    if output.status.success() {
        let ppid_str = String::from_utf8_lossy(&output.stdout);
        ppid_str.trim().parse().ok()
    } else {
        None
    }
}

/// Check if a command looks like a renamed child process (e.g., "next-server (v16.0.7)")
fn is_renamed_child_process(cmd: &str) -> bool {
    // Common patterns for renamed child processes:
    // - Contains version in parentheses like "(v16.0.7)"
    // - Is just a simple name without a path or arguments
    // - Doesn't start with a path or common executable

    // Check for version pattern like "(vX.X.X)" or "(version)"
    if cmd.contains("(v") && cmd.contains(")") {
        return true;
    }

    // Check if it's a simple name without any path separators or meaningful args
    let first_arg = cmd.split_whitespace().next().unwrap_or("");
    if !first_arg.contains('/') && !first_arg.ends_with(".js") && !first_arg.ends_with(".ts") {
        // If first argument is not a path and cmd is very short, likely renamed
        if cmd.len() < 50 && !cmd.contains("--") && !cmd.contains("-p") {
            return true;
        }
    }

    false
}

/// Try to get the best command line by walking up the process tree if needed
fn get_best_cmdline_for_pid(pid: i32) -> Option<String> {
    let cmd = get_cmdline_from_pid(pid)?;

    // If the command looks like a renamed child process, try the parent
    if is_renamed_child_process(&cmd) {
        if let Some(ppid) = get_ppid(pid as u32) {
            // Don't go above PID 1 (init) and avoid very low PIDs
            if ppid > 1 {
                if let Some(parent_cmd) = get_cmdline_from_pid(ppid as i32) {
                    // Only use parent command if it looks more meaningful
                    if !is_renamed_child_process(&parent_cmd) {
                        return Some(parent_cmd);
                    }
                }
            }
        }
    }

    Some(cmd)
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
