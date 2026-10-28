mod platform;
mod schedule;
mod sound;
mod state;

use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri_plugin_notification::NotificationExt;

use chrono::{Datelike, Timelike};

use state::{Pos, Settings};

fn widget_frame(size: &str) -> (f64, f64) {
    match size {
        "medium" => (178.0, 96.0),
        "small" => (134.0, 74.0),
        _ => (234.0, 120.0),
    }
}

/// Same curve as the card, plus the transparent margin, so the halo is concentric.
fn widget_radius(size: &str) -> f64 {
    match size {
        "medium" => 26.0,
        "small" => 22.0,
        _ => 30.0,
    }
}

const PANEL_W: f64 = 436.0;
const PANEL_H: f64 = 696.0;
const PANEL_RADIUS: f64 = 36.0;
const GAP: f64 = 12.0;
const SHOUT_W: f64 = 240.0;
const SHOUT_H: f64 = 110.0;
const SHOUT_VERTICAL_MARGIN: f64 = 12.0;

static NEXT_ID: AtomicI32 = AtomicI32::new(1);

struct Store {
    path: PathBuf,
    data: Mutex<state::AppData>,
}

struct TrayDnd(CheckMenuItem<tauri::Wry>);

struct TrayWidgetVisibility(MenuItem<tauri::Wry>);

struct ShoutHold {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// Logical x of the resting widget's center, so the bubble can open away from the nearest edge.
    center: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ShoutLayout {
    side: String,
    vertical: String,
}

impl Clone for ShoutHold {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for ShoutHold {}

struct Shout(Mutex<Option<ShoutHold>>);

struct Area {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Clone for Area {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Area {}

pub fn run() {
    platform::prepare_session();
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_state,
            toggle_panel,
            hide_panel,
            quit,
            import_sound,
            preview_sound,
            remove_sound,
            peek_shout_layout,
            begin_shout,
            end_shout,
            preview_reaction
        ])
        .setup(|app| {
            setup(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("no pude abrir Gentle Break")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(ActivationPolicy::Accessory);

    let path = app.path().app_data_dir()?.join("state.json");
    let data = state::load(&path);
    let dnd = data.dnd;
    let widget_visible = data.widget_visible;
    let saved = data.widget.clone();
    let widget_size = data.widget_size.clone();
    app.manage(Store { path, data: Mutex::new(data) });
    app.manage(Shout(Mutex::new(None)));
    sync_bundled_sounds(app.handle());
    build_windows(app.handle(), saved, &widget_size, widget_visible)?;
    build_tray(app.handle(), dnd, widget_visible)?;
    spawn_scheduler(app.handle().clone());
    Ok(())
}

fn build_windows(app: &AppHandle, saved: Option<Pos>, widget_size: &str, widget_visible: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = widget_frame(widget_size);
    let widget = WebviewWindowBuilder::new(app, "widget", WebviewUrl::App("index.html?surface=widget".into()))
        .title("Gentle Break")
        .inner_size(width, height)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .shadow(false)
        .visible(false);
    #[cfg(target_os = "macos")]
    let widget = widget.accept_first_mouse(true);
    // GTK sizes a non-resizable window to its content, so the min and max pin it.
    #[cfg(target_os = "linux")]
    let widget = widget.resizable(true).min_inner_size(width, height).max_inner_size(width, height);
    let widget = widget.build()?;
    round_window(&widget, widget_radius(widget_size));

    let panel = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("index.html?surface=panel".into()))
        .title("Gentle Break")
        .inner_size(PANEL_W, PANEL_H)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false);
    #[cfg(target_os = "macos")]
    let panel = panel.accept_first_mouse(true);
    #[cfg(target_os = "linux")]
    let panel = panel
        .resizable(true)
        .min_inner_size(PANEL_W, PANEL_H)
        .max_inner_size(PANEL_W, PANEL_H);
    let panel = panel.build()?;
    round_window(&panel, PANEL_RADIUS);

    let places_windows = platform::app_places_windows(|key| std::env::var(key).ok());
    if places_windows {
        let (x, y) = initial_widget(&widget, app, saved, width, height);
        widget.set_position(LogicalPosition::new(x, y))?;
    } else {
        eprintln!("gentle-break: este escritorio coloca la ventana; en Wayland nativo no se puede fijar el reloj");
    }
    widget.set_size(LogicalSize::new(width, height))?;

    let (tx, rx) = mpsc::channel::<(f64, f64)>();
    let move_app = app.clone();
    widget.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
        WindowEvent::Moved(_) => {
            if move_app.try_state::<Shout>().is_some_and(|shout| shout.0.lock().ok().is_some_and(|hold| hold.is_some())) {
                return;
            }
            if let Some(window) = move_app.get_webview_window("widget") {
                if let (Ok(pos), Ok(scale)) = (window.outer_position(), window.scale_factor()) {
                    if scale > 0.0 {
                        let _ = tx.send((pos.x as f64 / scale, pos.y as f64 / scale));
                    }
                }
            }
        }
        _ => {}
    });

    let panel_app = app.clone();
    if let Some(panel) = app.get_webview_window("panel") {
        panel.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = hide_panel_window(&panel_app);
            }
        });
    }

    if widget_visible {
        widget.show()?;
    }
    let saver = app.clone();
    thread::spawn(move || {
        while let Ok(pos) = rx.recv() {
            let mut latest = pos;
            loop {
                match rx.recv_timeout(Duration::from_millis(350)) {
                    Ok(pos) => latest = pos,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            save_widget_pos(&saver, latest.0, latest.1);
        }
    });
    Ok(())
}

fn build_tray(app: &AppHandle, dnd: bool, widget_visible: bool) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let visibility = MenuItem::with_id(
        app,
        "visibility",
        widget_visibility_label(widget_visible),
        true,
        None::<&str>,
    )?;
    let dnd_item = CheckMenuItem::with_id(app, "dnd", "No molestar", true, dnd, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &visibility, &dnd_item, &quit])?;
    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
    app.manage(TrayDnd(dnd_item));
    app.manage(TrayWidgetVisibility(visibility));
    let tray = TrayIconBuilder::with_id("tray")
        .tooltip("Gentle Break")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Err(error) = show_panel(app) {
                    eprintln!("gentle-break: {error}");
                }
            }
            "quit" => app.exit(0),
            "visibility" => toggle_widget_visibility(app),
            "dnd" => toggle_dnd(app),
            _ => {}
        });
    #[cfg(target_os = "macos")]
    let tray = tray.icon_as_template(true);
    tray.build(app)?;
    Ok(())
}

fn spawn_scheduler(app: AppHandle) {
    thread::spawn(move || loop {
        tick(&app);
        thread::sleep(Duration::from_secs(1));
    });
}

fn tick(app: &AppHandle) {
    let store = app.state::<Store>();
    let pending = {
        let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        let tick = schedule::evaluate(&data, moment_now());
        if tick.notices.is_empty() {
            return;
        }
        for notice in &tick.notices {
            schedule::mark(&mut data, notice);
        }
        if let Err(error) = state::store(&store.path, &data) {
            eprintln!("gentle-break: no pude guardar el aviso: {error}");
        }
        (tick.notices, tick.audible)
    };
    let (notices, audible) = pending;
    if !audible {
        return;
    }
    let sounds_dir = sounds_directory(app);
    let (customs, reaction) = {
        let store = app.state::<Store>();
        let data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        (data.sounds.clone(), data.reaction.clone())
    };
    for notice in &notices {
        let choice = sound::reaction_sound_id(&reaction).unwrap_or(notice.sound.as_str());
        play_choice(app, &sounds_dir, &customs, choice);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if let Err(error) = app
            .notification()
            .builder()
            .id(id)
            .title(&notice.title)
            .body(&notice.body)
            .sound(alert_sound())
            .show()
        {
            eprintln!("gentle-break: notificación: {error}");
        }
    }
    let title = notices.iter().map(|notice| notice.title.as_str()).collect::<Vec<_>>().join(" · ");
    let body = notices
        .iter()
        .find(|notice| notice.kind == schedule::Kind::Break)
        .map(|notice| notice.body.clone())
        .unwrap_or_default();
    let _ = app.emit("gentle-break://fired", serde_json::json!({ "title": title, "body": body }));
}

fn alert_sound() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "NSUserNotificationDefaultSoundName"
    }
    #[cfg(windows)]
    {
        "Default"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "message-new-instant"
    }
}

/// The banner sound is unreliable, so each OS also plays its own short chime.
fn play_chime() {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("/usr/bin/afplay");
    #[cfg(target_os = "macos")]
    command.arg("/System/Library/Sounds/Glass.aiff");
    #[cfg(windows)]
    let mut command = std::process::Command::new("powershell");
    #[cfg(windows)]
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[System.Media.SystemSounds]::Asterisk.Play()",
    ]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("canberra-gtk-play");
    #[cfg(all(unix, not(target_os = "macos")))]
    command.args(["-i", "bell"]);
    let _ = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn moment_now() -> schedule::Moment {
    let now = chrono::Local::now();
    schedule::Moment {
        weekday: now.weekday().number_from_monday() as u8,
        minutes: (now.hour() * 60 + now.minute()) as u16,
        date: now.format("%Y-%m-%d").to_string(),
    }
}

fn save_widget_pos(app: &AppHandle, x: f64, y: f64) {
    let store = app.state::<Store>();
    let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
    data.widget = Some(Pos { x, y });
    if let Err(error) = state::store(&store.path, &data) {
        eprintln!("gentle-break: no pude guardar la posición: {error}");
    }
}

fn widget_visibility_label(visible: bool) -> &'static str {
    if visible {
        "Ocultar widget"
    } else {
        "Mostrar widget"
    }
}

fn toggle_dnd(app: &AppHandle) {
    let store = app.state::<Store>();
    let (dnd, visible) = {
        let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        data.dnd = !data.dnd;
        if let Err(error) = state::store(&store.path, &data) {
            eprintln!("gentle-break: {error}");
        }
        (data.dnd, data.widget_visible)
    };
    sync_tray(app, dnd, visible);
    emit_live(app);
}

fn toggle_widget_visibility(app: &AppHandle) {
    let store = app.state::<Store>();
    let (dnd, visible) = {
        let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        data.widget_visible = !data.widget_visible;
        if let Err(error) = state::store(&store.path, &data) {
            eprintln!("gentle-break: {error}");
        }
        let visible = data.widget_visible;
        if let Err(error) = apply_widget_visibility(app, visible) {
            eprintln!("gentle-break: {error}");
        }
        (data.dnd, visible)
    };
    sync_tray(app, dnd, visible);
    emit_live(app);
}

fn apply_widget_visibility(app: &AppHandle, visible: bool) -> Result<(), String> {
    let widget = webview(app, "widget")?;
    if visible {
        widget.show().map_err(|error| error.to_string())?;
    } else {
        widget.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn sync_tray(app: &AppHandle, dnd: bool, widget_visible: bool) {
    if let Some(item) = app.try_state::<TrayDnd>() {
        let _ = item.0.set_checked(dnd);
    }
    if let Some(item) = app.try_state::<TrayWidgetVisibility>() {
        let _ = item.0.set_text(widget_visibility_label(widget_visible));
    }
}

fn show_panel(app: &AppHandle) -> Result<(), String> {
    place_panel(app)?;
    let panel = webview(app, "panel")?;
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Regular);
    panel.show().map_err(|error| error.to_string())?;
    let _ = panel.set_focus();
    Ok(())
}

fn hide_panel_window(app: &AppHandle) -> Result<(), String> {
    let panel = webview(app, "panel")?;
    panel.hide().map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    Ok(())
}

fn widget_points(widget: &WebviewWindow) -> (f64, f64) {
    let scale = widget.scale_factor().unwrap_or(1.0).max(1.0);
    widget
        .inner_size()
        .ok()
        .map(|size| (size.width as f64 / scale, size.height as f64 / scale))
        .unwrap_or_else(|| widget_frame("large"))
}

fn place_panel(app: &AppHandle) -> Result<(), String> {
    let widget = webview(app, "widget")?;
    let panel = webview(app, "panel")?;
    let scale = widget.scale_factor().unwrap_or(1.0).max(1.0);
    let pos = widget.outer_position().map_err(|error| error.to_string())?;
    let wx = pos.x as f64 / scale;
    let wy = pos.y as f64 / scale;
    let (widget_w, widget_h) = widget_points(&widget);
    let areas = areas_of(&widget);
    let area = area_containing(&areas, wx + widget_w / 2.0, wy + widget_h / 2.0);
    let mut x = wx - PANEL_W - GAP;
    if x < area.x + 8.0 {
        x = wx + widget_w + GAP;
    }
    let max_x = (area.x + area.w - PANEL_W - 8.0).max(area.x + 8.0);
    let max_y = (area.y + area.h - PANEL_H - 8.0).max(area.y + 8.0);
    x = x.clamp(area.x + 8.0, max_x);
    let y = (wy + widget_h / 2.0 - PANEL_H / 2.0).clamp(area.y + 8.0, max_y);
    panel.set_position(LogicalPosition::new(x, y)).map_err(|error| error.to_string())?;
    Ok(())
}

fn initial_widget(window: &WebviewWindow, app: &AppHandle, saved: Option<Pos>, width: f64, height: f64) -> (f64, f64) {
    let areas = areas_of(window);
    if let Some(pos) = saved {
        if areas.iter().any(|area| intersects(pos.x, pos.y, width, height, area)) {
            return (pos.x, pos.y);
        }
    }
    let primary = app.primary_monitor().ok().flatten().map(|monitor| area_of(&monitor));
    let area = primary.or_else(|| areas.first().copied()).unwrap_or(Area { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 });
    (area.x + area.w - width - 24.0, area.y + (area.h - height) / 2.0)
}

fn areas_of(window: &WebviewWindow) -> Vec<Area> {
    window.available_monitors().unwrap_or_default().iter().map(area_of).collect()
}

fn area_of(monitor: &tauri::Monitor) -> Area {
    let scale = monitor.scale_factor().max(1.0);
    let rect = monitor.work_area();
    Area {
        x: rect.position.x as f64 / scale,
        y: rect.position.y as f64 / scale,
        w: rect.size.width as f64 / scale,
        h: rect.size.height as f64 / scale,
    }
}

fn area_containing(areas: &[Area], x: f64, y: f64) -> Area {
    areas
        .iter()
        .find(|area| x >= area.x && x < area.x + area.w && y >= area.y && y < area.y + area.h)
        .copied()
        .or_else(|| areas.first().copied())
        .unwrap_or(Area { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 })
}

fn intersects(x: f64, y: f64, w: f64, h: f64, area: &Area) -> bool {
    x < area.x + area.w && x + w > area.x && y < area.y + area.h && y + h > area.y
}

fn webview(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label).ok_or_else(|| format!("falta la ventana {label}"))
}

fn sounds_directory(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().map(|dir| dir.join("sounds")).unwrap_or_else(|_| std::path::PathBuf::from("sounds"))
}

fn play_choice(app: &AppHandle, dir: &std::path::Path, customs: &[state::CustomSound], choice: &str) {
    let resolved = if choice == "random" {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() as u64)
            .unwrap_or(0);
        sound::random_preset(seed)
    } else {
        choice
    };
    if let Some(path) = sound_file(app, dir, customs, resolved) {
        play_file(&path);
    } else {
        play_chime();
    }
}

/// Copia los MP3 embebidos en cada arranque para que un rebuild se escuche al toque.
fn sync_bundled_sounds(app: &AppHandle) {
    let dir = sounds_directory(app);
    let _ = std::fs::create_dir_all(&dir);
    for id in [sound::MUSTACHE_ID, sound::ABUELA_ID] {
        if let Some(bytes) = sound::bundled_bytes(id) {
            let path = dir.join(format!(".bundled-{id}.mp3"));
            let _ = std::fs::write(path, bytes);
        }
    }
}

fn bundled_sound_path(app: &AppHandle, id: &str) -> Option<std::path::PathBuf> {
    if !sound::is_bundled_id(id) {
        return None;
    }
    Some(sounds_directory(app).join(format!(".bundled-{id}.mp3")))
}

fn sound_file(app: &AppHandle, dir: &std::path::Path, customs: &[state::CustomSound], choice: &str) -> Option<std::path::PathBuf> {
    if let Some(path) = bundled_sound_path(app, choice) {
        return Some(path);
    }
    if let Some(id) = choice.strip_prefix("custom:") {
        let item = customs.iter().find(|sound| sound.id == id)?;
        let path = dir.join(&item.file);
        return path.is_file().then_some(path);
    }
    let file = sound::preset_file(choice)?;
    #[cfg(target_os = "macos")]
    {
        let path = std::path::PathBuf::from(format!("/System/Library/Sounds/{file}.aiff"));
        return path.is_file().then_some(path);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (file, dir);
        None
    }
}

fn play_file(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("/usr/bin/afplay")
            .arg(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        play_chime();
    }
}

fn check_duration(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/afinfo").arg(path).output().map_err(|_| "No pude leer el audio.".to_string())?;
        if !output.status.success() {
            return Err("Ese archivo no es un audio que pueda sonar.".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let seconds = sound::duration_from_afinfo(&text).ok_or("No pude leer la duración.")?;
        if seconds > sound::MAX_SOUND_SECONDS {
            return Err("El sonido dura más de 15 segundos.".into());
        }
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Ok(())
    }
}

#[tauri::command]
fn import_sound(app: AppHandle, path: String) -> Result<state::CustomSound, String> {
    let source = std::path::PathBuf::from(&path);
    let filename = source.file_name().and_then(|name| name.to_str()).unwrap_or("sonido");
    let extension = sound::allowed_extension(filename).ok_or("Usá WAV, MP3, M4A, AIFF o CAF.")?;
    let meta = std::fs::metadata(&source).map_err(|_| "No pude leer el archivo.".to_string())?;
    if !meta.is_file() {
        return Err("Elegí un archivo de audio.".into());
    }
    if meta.len() > sound::MAX_SOUND_BYTES {
        return Err("El archivo pesa más de 2 MB.".into());
    }
    check_duration(&source)?;
    let id = format!(
        "{:x}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_nanos()).unwrap_or(0)
    );
    let file = format!("{id}.{extension}");
    let dir = sounds_directory(&app);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    std::fs::copy(&source, dir.join(&file)).map_err(|error| error.to_string())?;
    let stem = filename.rsplit_once('.').map(|(name, _)| name).unwrap_or(filename);
    let custom = state::CustomSound {
        id: id.clone(),
        name: stem.chars().take(80).collect(),
        file,
    };
    let store = app.state::<Store>();
    let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
    data.sounds.push(custom.clone());
    state::store(&store.path, &data)?;
    Ok(custom)
}

#[tauri::command]
fn preview_sound(app: AppHandle, sound: String) -> Result<(), String> {
    let store = app.state::<Store>();
    let customs = {
        let data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        data.sounds.clone()
    };
    play_choice(&app, &sounds_directory(&app), &customs, &sound);
    Ok(())
}

#[tauri::command]
fn remove_sound(app: AppHandle, id: String) -> Result<(), String> {
    let store = app.state::<Store>();
    let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
    let Some(index) = data.sounds.iter().position(|sound| sound.id == id) else {
        return Ok(());
    };
    let removed = data.sounds.remove(index);
    let file = sounds_directory(&app).join(&removed.file);
    if file.is_file() {
        let _ = std::fs::remove_file(file);
    }
    let key = format!("custom:{id}");
    for alarm in &mut data.alarms {
        if alarm.sound == key {
            alarm.sound = "glass".into();
        }
    }
    for item in &mut data.breaks {
        if item.sound == key {
            item.sound = "ping".into();
        }
    }
    state::store(&store.path, &data)?;
    Ok(())
}

#[tauri::command]
fn get_state(app: AppHandle) -> Settings {
    let store = app.state::<Store>();
    let data = store.data.lock().unwrap_or_else(|error| error.into_inner());
    state::settings_of(&data)
}

#[tauri::command]
fn save_state(app: AppHandle, settings: Settings) -> Result<(), String> {
    let store = app.state::<Store>();
    let (dnd, previous, next, widget_visible) = {
        let mut data = store.data.lock().unwrap_or_else(|error| error.into_inner());
        let previous = data.widget_size.clone();
        state::apply_settings(&mut data, settings);
        state::store(&store.path, &data)?;
        (data.dnd, previous, data.widget_size.clone(), data.widget_visible)
    };
    if previous != next {
        resize_widget(&app, &next);
    }
    if let Err(error) = apply_widget_visibility(&app, widget_visible) {
        eprintln!("gentle-break: {error}");
    }
    sync_tray(&app, dnd, widget_visible);
    emit_live(&app);
    Ok(())
}

fn resize_widget(app: &AppHandle, size: &str) {
    let Some(widget) = app.get_webview_window("widget") else {
        return;
    };
    let (width, height) = widget_frame(size);
    if let Err(error) = widget.set_size(LogicalSize::new(width, height)) {
        eprintln!("gentle-break: no pude cambiar el tamaño: {error}");
    }
    round_window(&widget, widget_radius(size));
}

/// Opens the bubble on the side that has more room: left when the widget sits on the right half.
fn bubble_side(center: f64, middle: f64) -> &'static str {
    if center > middle { "left" } else { "right" }
}

/// Prefers opening above the clock when there is room; only opens below when the top is tight.
fn bubble_vertical_prefer_above(space_above: f64, space_below: f64) -> &'static str {
    let need = SHOUT_H + SHOUT_VERTICAL_MARGIN;
    if space_above >= need {
        "above"
    } else if space_below >= need {
        "below"
    } else if space_above >= space_below {
        "above"
    } else {
        "below"
    }
}

fn shout_vertical(window: &WebviewWindow) -> Option<&'static str> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
    let pos = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    let top = pos.y as f64 / scale;
    let bottom = top + size.height as f64 / scale;
    let mon_scale = monitor.scale_factor().max(1.0);
    let mon_top = monitor.position().y as f64 / mon_scale;
    let mon_bottom = mon_top + monitor.size().height as f64 / mon_scale;
    let space_above = top - mon_top;
    let space_below = mon_bottom - bottom;
    Some(bubble_vertical_prefer_above(space_above, space_below))
}

fn monitor_middle(window: &WebviewWindow) -> Option<f64> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor().max(1.0);
    Some(monitor.position().x as f64 / scale + monitor.size().width as f64 / scale / 2.0)
}

fn capture_frame(window: &WebviewWindow) -> Result<ShoutHold, String> {
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
    let pos = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let logical_x = pos.x as f64 / scale;
    let logical_w = size.width as f64 / scale;
    #[cfg(target_os = "macos")]
    {
        let frame = cocoa_frame(window)?;
        return Ok(ShoutHold {
            x: frame.origin.x,
            y: frame.origin.y,
            w: frame.size.width,
            h: frame.size.height,
            center: logical_x + logical_w / 2.0,
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let logical_y = pos.y as f64 / scale;
        let logical_h = size.height as f64 / scale;
        Ok(ShoutHold {
            x: logical_x,
            y: logical_y,
            w: logical_w,
            h: logical_h,
            center: logical_x + logical_w / 2.0,
        })
    }
}

fn resting_hold(app: &AppHandle, window: &WebviewWindow) -> Result<ShoutHold, String> {
    let shout = app.state::<Shout>();
    let mut slot = shout.0.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(hold) = *slot {
        return Ok(hold);
    }
    let hold = capture_frame(window)?;
    *slot = Some(hold);
    Ok(hold)
}

/// Grows the window in one step. On macOS `setContentSize` grows upward and `set_position`
/// also moves the top, so calling both shifts the widget off the spot and clips the mustache.
fn apply_shout_frame(window: &WebviewWindow, resting: &ShoutHold, side: &str, vertical: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let origin_x = if side == "left" { resting.x - SHOUT_W } else { resting.x };
        let origin_y = if vertical == "below" {
            resting.y - SHOUT_H
        } else {
            resting.y
        };
        let frame = cocoa_rect(origin_x, origin_y, resting.w + SHOUT_W, resting.h + SHOUT_H);
        set_cocoa_frame(window, frame)?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_resizable(true);
        let width = resting.w + SHOUT_W;
        let height = resting.h + SHOUT_H;
        let _ = window.set_min_size(Some(LogicalSize::new(width, height)));
        let _ = window.set_max_size(Some(LogicalSize::new(width, height)));
        window.set_size(LogicalSize::new(width, height)).map_err(|error| error.to_string())?;
        let x = if side == "left" { resting.x - SHOUT_W } else { resting.x };
        let y = if vertical == "below" { resting.y } else { resting.y - SHOUT_H };
        window
            .set_position(LogicalPosition::new(x, y))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn restore_frame(window: &WebviewWindow, resting: &ShoutHold) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        set_cocoa_frame(window, cocoa_rect(resting.x, resting.y, resting.w, resting.h))?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_min_size(Some(LogicalSize::new(resting.w, resting.h)));
        let _ = window.set_max_size(Some(LogicalSize::new(resting.w, resting.h)));
        window.set_size(LogicalSize::new(resting.w, resting.h)).map_err(|error| error.to_string())?;
        window
            .set_position(LogicalPosition::new(resting.x, resting.y))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn compute_shout_layout(window: &WebviewWindow, hold: &ShoutHold) -> ShoutLayout {
    let side = match monitor_middle(window) {
        Some(middle) => bubble_side(hold.center, middle).to_string(),
        None => "right".into(),
    };
    let vertical = shout_vertical(window).unwrap_or("above").to_string();
    ShoutLayout { side, vertical }
}

#[tauri::command]
fn peek_shout_layout(app: AppHandle) -> Result<ShoutLayout, String> {
    let widget = webview(&app, "widget")?;
    let hold = capture_frame(&widget)?;
    Ok(compute_shout_layout(&widget, &hold))
}

#[tauri::command]
fn begin_shout(app: AppHandle) -> Result<ShoutLayout, String> {
    let widget = webview(&app, "widget")?;
    let resting = resting_hold(&app, &widget)?;
    let layout = compute_shout_layout(&widget, &resting);
    apply_shout_frame(&widget, &resting, &layout.side, &layout.vertical)?;
    // Solo quitamos la máscara: set_cocoa_frame ya estiró el webview; otro stretch parpadea.
    set_corner_mask(&widget, 0.0);
    Ok(layout)
}

#[tauri::command]
fn end_shout(app: AppHandle) -> Result<(), String> {
    let widget = webview(&app, "widget")?;
    let hold = {
        let shout = app.state::<Shout>();
        let mut guard = shout.0.lock().unwrap_or_else(|error| error.into_inner());
        guard.take()
    };
    let Some(hold) = hold else {
        return Ok(());
    };
    restore_frame(&widget, &hold)?;
    let store = app.state::<Store>();
    let size = store.data.lock().unwrap_or_else(|error| error.into_inner()).widget_size.clone();
    set_corner_mask(&widget, widget_radius(&size));
    Ok(())
}

#[tauri::command]
fn preview_reaction(app: AppHandle, reaction: String, title: String, body: String) -> Result<(), String> {
    let reaction = state::normalize_reaction(&reaction);
    if reaction == "none" {
        return Ok(());
    }
    if let Some(sound_id) = sound::reaction_sound_id(&reaction) {
        let store = app.state::<Store>();
        let customs = store.data.lock().unwrap_or_else(|error| error.into_inner()).sounds.clone();
        play_choice(&app, &sounds_directory(&app), &customs, sound_id);
    }
    app.emit(
        "gentle-break://preview",
        serde_json::json!({ "reaction": reaction, "title": title, "body": body }),
    )
    .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn cocoa_window(window: &WebviewWindow) -> Result<*mut objc2::runtime::AnyObject, String> {
    let pointer = window.ns_window().map_err(|error| error.to_string())?;
    if pointer.is_null() {
        return Err("sin ventana nativa".into());
    }
    Ok(pointer as *mut objc2::runtime::AnyObject)
}

#[cfg(target_os = "macos")]
fn cocoa_rect(x: f64, y: f64, width: f64, height: f64) -> objc2_foundation::NSRect {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

#[cfg(target_os = "macos")]
fn cocoa_frame(window: &WebviewWindow) -> Result<objc2_foundation::NSRect, String> {
    use objc2::msg_send;
    let window = cocoa_window(window)?;
    Ok(unsafe { msg_send![window, frame] })
}

#[cfg(target_os = "macos")]
fn set_cocoa_frame(window: &WebviewWindow, frame: objc2_foundation::NSRect) -> Result<(), String> {
    use objc2::msg_send;
    use objc2::runtime::Bool;
    use objc2_foundation::NSSize;
    let ns_window = cocoa_window(window)?;
    unsafe {
        // A leftover max size would ignore the taller frame and clip the mustache.
        let _: () = msg_send![ns_window, setContentMinSize: NSSize::new(1.0, 1.0)];
        let _: () = msg_send![ns_window, setContentMaxSize: NSSize::new(4000.0, 4000.0)];
        let _: () = msg_send![ns_window, setFrame: frame, display: Bool::YES];
    }
    stretch_webview(window);
    Ok(())
}

/// WKWebView keeps its old size unless we resize it to the content view bounds.
#[cfg(target_os = "macos")]
fn stretch_webview(window: &WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::NSRect;

    let Ok(ns_window) = cocoa_window(window) else {
        return;
    };
    unsafe {
        let content: *mut AnyObject = msg_send![ns_window, contentView];
        if content.is_null() {
            return;
        }
        let bounds: NSRect = msg_send![content, bounds];
        let subviews: *mut AnyObject = msg_send![content, subviews];
        if subviews.is_null() {
            return;
        }
        let count: usize = msg_send![subviews, count];
        for index in 0..count {
            let child: *mut AnyObject = msg_send![subviews, objectAtIndex: index];
            if !child.is_null() {
                let _: () = msg_send![child, setFrame: bounds];
            }
        }
    }
}

/// Clips the macOS window to the same rounded rect as the card, so the halo
/// does not end in the window's sharp corners. Also stretches the webview to the
/// new frame: otherwise the extra space stays empty and the mustache never shows.
#[cfg(target_os = "macos")]
fn round_window(window: &WebviewWindow, radius: f64) {
    stretch_webview(window);
    set_corner_mask(window, radius);
}

#[cfg(target_os = "macos")]
fn set_corner_mask(window: &WebviewWindow, radius: f64) {
    use objc2::msg_send;
    use objc2::runtime::{AnyObject, Bool};
    let Ok(ns_window) = cocoa_window(window) else {
        return;
    };
    unsafe {
        let content: *mut AnyObject = msg_send![ns_window, contentView];
        if content.is_null() {
            return;
        }
        let _: () = msg_send![content, setWantsLayer: Bool::YES];
        let layer: *mut AnyObject = msg_send![content, layer];
        if !layer.is_null() {
            if radius > 0.0 {
                let _: () = msg_send![layer, setCornerRadius: radius];
                let _: () = msg_send![layer, setMasksToBounds: Bool::YES];
            } else {
                let _: () = msg_send![layer, setMasksToBounds: Bool::NO];
            }
        }
        let _: () = msg_send![ns_window, setHasShadow: Bool::NO];
        let _: () = msg_send![ns_window, invalidateShadow];
    }
}

#[cfg(not(target_os = "macos"))]
fn round_window(_window: &WebviewWindow, _radius: f64) {}

#[cfg(not(target_os = "macos"))]
fn set_corner_mask(_window: &WebviewWindow, _radius: f64) {}

fn emit_live(app: &AppHandle) {
    let store = app.state::<Store>();
    let data = store.data.lock().unwrap_or_else(|error| error.into_inner());
    let _ = app.emit(
        "gentle-break://live",
        serde_json::json!({
            "dnd": data.dnd,
            "widgetSize": data.widget_size,
            "widgetVisible": data.widget_visible,
            "reaction": data.reaction,
        }),
    );
}

#[tauri::command]
fn toggle_panel(app: AppHandle) -> Result<(), String> {
    let panel = webview(&app, "panel")?;
    if panel.is_visible().unwrap_or(false) {
        hide_panel_window(&app)
    } else {
        show_panel(&app)
    }
}

#[tauri::command]
fn hide_panel(app: AppHandle) -> Result<(), String> {
    hide_panel_window(&app)
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::{bubble_side, bubble_vertical_prefer_above, SHOUT_H, SHOUT_VERTICAL_MARGIN};

    #[test]
    fn bubble_opens_toward_the_free_side() {
        assert_eq!(bubble_side(1200.0, 800.0), "left");
        assert_eq!(bubble_side(200.0, 800.0), "right");
        assert_eq!(bubble_side(800.0, 800.0), "right");
    }

    #[test]
    fn bubble_prefers_above_when_there_is_room() {
        let need = SHOUT_H + SHOUT_VERTICAL_MARGIN;
        assert_eq!(bubble_vertical_prefer_above(need + 40.0, need + 200.0), "above");
        assert_eq!(bubble_vertical_prefer_above(need + 10.0, need + 10.0), "above");
    }

    #[test]
    fn bubble_opens_below_only_when_the_top_is_tight() {
        let need = SHOUT_H + SHOUT_VERTICAL_MARGIN;
        assert_eq!(bubble_vertical_prefer_above(need - 20.0, need + 80.0), "below");
        assert_eq!(bubble_vertical_prefer_above(40.0, 200.0), "below");
    }
}
