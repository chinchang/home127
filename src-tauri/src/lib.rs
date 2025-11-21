mod scanner;

use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
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
                                let window_width = 300.0;
                                
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
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, scanner::scan_servers])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

