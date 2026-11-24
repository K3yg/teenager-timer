//! Stick Timer desktop shell.
//!
//! The UI is one transparent, always-on-top window covering the work area of the
//! primary monitor (so the dial can be dragged, tucked into edges and corners, and the
//! menu can bloom exactly like the web prototype). To keep the rest of the desktop
//! usable, the window ignores the mouse everywhere except the "hit" shapes the UI
//! reports (dial circle, menu card, orbit buttons). A light poller flips
//! `set_ignore_cursor_events` only when the cursor crosses one of those shapes.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// A shape in CSS pixels, relative to the window's top-left corner.
#[derive(Clone, Copy, Debug, Deserialize)]
struct Hit {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    round: bool,
}

impl Hit {
    fn contains(&self, px: f64, py: f64) -> bool {
        if self.round {
            let (rx, ry) = (self.w / 2.0, self.h / 2.0);
            let (dx, dy) = ((px - (self.x + rx)) / rx, (py - (self.y + ry)) / ry);
            dx * dx + dy * dy <= 1.0
        } else {
            px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
        }
    }
}

#[derive(Default)]
struct HitState {
    shapes: Vec<Hit>,
    /// While a drag/turn is in progress the window keeps the mouse, even if the
    /// cursor briefly outruns the reported shapes.
    hold: bool,
}

type Shared = Arc<Mutex<HitState>>;

#[tauri::command]
fn set_hit_shapes(state: State<'_, Shared>, shapes: Vec<Hit>) {
    state.lock().unwrap().shapes = shapes;
}

#[tauri::command]
fn set_hold(state: State<'_, Shared>, hold: bool) {
    state.lock().unwrap().hold = hold;
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// Cover the primary monitor's work area (screen minus taskbar/Dock).
fn fit_to_work_area(win: &WebviewWindow) {
    let monitor = win.primary_monitor().ok().flatten().or_else(|| win.current_monitor().ok().flatten());
    if let Some(m) = monitor {
        let area = m.work_area();
        let _ = win.set_position(area.position);
        let _ = win.set_size(area.size);
    }
}

fn toggle_visible(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(true) {
            let _ = w.hide();
        } else {
            let _ = w.show();
        }
    }
}

/// Flip mouse pass-through when the cursor enters/leaves an interactive shape.
fn spawn_hit_poller(win: WebviewWindow, state: Shared) {
    std::thread::spawn(move || {
        let mut interactive: Option<bool> = None;
        loop {
            std::thread::sleep(Duration::from_millis(33));
            // If anything fails, fall back to pass-through: never block the desktop.
            let inside = match (win.cursor_position(), win.inner_position(), win.scale_factor()) {
                (Ok(cursor), Ok(origin), Ok(scale)) => {
                    let (lx, ly) = ((cursor.x - origin.x as f64) / scale, (cursor.y - origin.y as f64) / scale);
                    let s = state.lock().unwrap();
                    s.hold || s.shapes.iter().any(|h| h.contains(lx, ly))
                }
                _ => false,
            };
            if interactive != Some(inside) {
                match win.set_ignore_cursor_events(!inside) {
                    Ok(()) => interactive = Some(inside),
                    Err(e) => eprintln!("set_ignore_cursor_events failed: {e}"),
                }
                let _ = win.emit("mouse-zone", inside);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared: Shared = Arc::new(Mutex::new(HitState::default()));

    tauri::Builder::default()
        // Opening the app again just brings the dial back.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    // Space (Windows) and P (macOS) both start/pause.
                    let name = match shortcut.key {
                        Code::KeyN => "quick-add",
                        Code::KeyK => "summon",
                        _ => "toggle-timer",
                    };
                    // Typing needs keyboard focus: bring the window forward for N / K.
                    if name != "toggle-timer" {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    let _ = app.emit(name, ());
                })
                .build(),
        )
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![set_hit_shapes, set_hold, quit])
        .setup(move |app| {
            // macOS: no Dock icon / Cmd+Tab entry; the app lives in the menu bar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let win = app.get_webview_window("main").expect("main window");
            fit_to_work_area(&win);
            // macOS: follow the user across Spaces and over full-screen apps.
            #[cfg(target_os = "macos")]
            let _ = win.set_visible_on_all_workspaces(true);
            win.show()?;
            // Pass-through from the very first frame; the poller takes over from here.
            win.set_ignore_cursor_events(true)?;
            spawn_hit_poller(win.clone(), shared.clone());

            // Global shortcuts.
            // Windows: Ctrl+Alt + Space (start/pause), N (new task), K (open in the middle).
            // macOS:   Cmd+Option + P (start/pause; Cmd+Option+Space is Finder search), N, K.
            #[cfg(target_os = "macos")]
            let (mods, keys) = (Modifiers::SUPER | Modifiers::ALT, [Code::KeyP, Code::KeyN, Code::KeyK]);
            #[cfg(not(target_os = "macos"))]
            let (mods, keys) = (Modifiers::CONTROL | Modifiers::ALT, [Code::Space, Code::KeyN, Code::KeyK]);
            for code in keys {
                if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(mods), code)) {
                    eprintln!("global shortcut {code:?} unavailable: {e}");
                }
            }

            let toggle = MenuItem::with_id(app, "toggle", "Show / hide", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;
            // macOS menu bar wants a monochrome template icon (tinted by the system).
            #[cfg(target_os = "macos")]
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
            #[cfg(not(target_os = "macos"))]
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            let tray = TrayIconBuilder::with_id("tray").icon(tray_icon);
            #[cfg(target_os = "macos")]
            let tray = tray.icon_as_template(true);
            tray
                .tooltip("Stick Timer")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "toggle" => toggle_visible(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_visible(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Stick Timer");
}
