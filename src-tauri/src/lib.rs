mod scanner;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn kill_server(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let feedback_i = MenuItem::with_id(app, "feedback", "Send feedback on chinchang457@gmail.com", true, None::<&str>)?;
            let about_i = MenuItem::with_id(app, "about", "Home127 v0.0.1 / Built by \"The CSSMonk\"", false, None::<&str>)?;
            let menu = Menu::with_items(app, &[&about_i, &feedback_i, &quit_i])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "feedback" => {
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg("echo 'chinchang457@gmail.com' | pbcopy")
                                .spawn();
                        }

                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // Toggle visibility logic
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                
                                // Calculate position
                                let window_width = 350.0;
                                
                                // Extract physical values from rect (assuming Physical for tray usually)
                                let (icon_x, icon_width) = match rect.position {
                                    tauri::Position::Physical(pos) => (pos.x as f64, match rect.size {
                                        tauri::Size::Physical(s) => s.width as f64,
                                        _ => 0.0,
                                    }),
                                    _ => (0.0, 0.0),
                                };
                                
                                let icon_y = match rect.position {
                                    tauri::Position::Physical(pos) => pos.y as f64,
                                    _ => 0.0,
                                };
                                
                                let icon_height = match rect.size {
                                    tauri::Size::Physical(s) => s.height as f64,
                                    _ => 0.0,
                                };

                                if icon_width > 0.0 {
                                    let x = icon_x + (icon_width / 2.0) - (window_width / 2.0);
                                    let y = icon_y + icon_height; // Below the icon

                                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                                        x: x as i32,
                                        y: y as i32,
                                    }));
                                }
                            }
                        }
                    }
                })
                .build(app)?;
            
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_shadow(false);
                let _ = window.set_visible_on_all_workspaces(true);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, scanner::scan_servers, kill_server])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

