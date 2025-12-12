mod scanner;

use tauri::menu::{Menu, MenuItem};
use tauri::path::BaseDirectory;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(MainPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

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

#[tauri::command]
fn start_server(cwd: String, command: String) -> Result<(), String> {
    // Use sh -c to execute the command string in the given directory
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                if !focused {
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let feedback_i = MenuItem::with_id(
                app,
                "feedback",
                "Send feedback on chinchang457@gmail.com",
                true,
                None::<&str>,
            )?;
            let about_i = MenuItem::with_id(
                app,
                "about",
                "Home127 v0.0.1 / Built by \"The CSSMonk\"",
                false,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&about_i, &feedback_i, &quit_i])?;

            #[cfg(target_os = "macos")]
            {
                let panel = PanelBuilder::<_, MainPanel>::new(app.handle(), "main")
                    .title("Home127")
                    .url(WebviewUrl::App("index.html".into()))
                    .size(tauri::Size::Logical(tauri::LogicalSize {
                        width: 350.0,
                        height: 400.0,
                    }))
                    .with_window(|w| {
                        w.decorations(false)
                            .transparent(true)
                            .visible(false)
                            .always_on_top(true)
                    })
                    .no_activate(true)
                    .level(PanelLevel::Status)
                    .collection_behavior(
                        CollectionBehavior::new()
                            .can_join_all_spaces()
                            .full_screen_auxiliary(),
                    )
                    .build();

                if let Err(e) = panel {
                    eprintln!("Failed to create panel: {}", e);
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let window =
                    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                        .title("Home127")
                        .inner_size(350.0, 400.0)
                        .decorations(false)
                        .transparent(true)
                        .visible(false)
                        .skip_taskbar(true)
                        .always_on_top(true)
                        .visible_on_all_workspaces(true)
                        .build();

                if let Err(e) = window {
                    eprintln!("Failed to create window: {}", e);
                }
            }

            // Load custom tray icon
            let tray_icon_path = app
                .path()
                .resolve("icons/system-tray-icon.png", BaseDirectory::Resource)?;

            // Load custom icon if it exists, otherwise use default
            let icon = if tray_icon_path.exists() {
                let img = image::open(&tray_icon_path)
                    .expect("Failed to open tray icon")
                    .to_rgba8();
                let (width, height) = img.dimensions();
                let rgba = img.into_raw();
                tauri::image::Image::new_owned(rgba, width, height)
            } else {
                app.default_window_icon().unwrap().clone()
            };

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
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
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // Toggle visibility logic
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                // Calculate position
                                let window_width = 350.0;

                                // Extract physical values from rect (assuming Physical for tray usually)
                                let (icon_x, icon_width) = match rect.position {
                                    tauri::Position::Physical(pos) => (
                                        pos.x as f64,
                                        match rect.size {
                                            tauri::Size::Physical(s) => s.width as f64,
                                            _ => 0.0,
                                        },
                                    ),
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

                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: x as i32,
                                            y: y as i32,
                                        },
                                    ));
                                }

                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            scanner::scan_servers,
            kill_server,
            start_server
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
