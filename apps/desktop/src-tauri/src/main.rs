#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod owned_process;

use owned_process::OwnedProcess;
use serde::Serialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use tauri::Manager;

#[derive(Clone, Serialize)]
struct Connection {
    endpoint: String,
    token: String,
}
struct Backend {
    child: Mutex<Option<OwnedProcess>>,
    connection: Connection,
}
impl Drop for Backend {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut()
            && let Some(process) = child.take()
        {
            drop(process);
        }
    }
}
#[tauri::command]
fn backend_connection(backend: tauri::State<'_, Backend>) -> Connection {
    backend.connection.clone()
}

/// Locate the packaged backend sidecar for every install layout.
///
/// Bundles keep the sidecar beside the GUI binary; the Tauri resource directory
/// (`/usr/lib/FlowGram` or `$APPDIR/usr/lib/FlowGram`) is also probed for
/// installers that relocate external binaries.
fn resolve_backend(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    if let Some(configured) = std::env::var_os("FLOWGRAM_SERVER_BIN") {
        candidates.push(PathBuf::from(configured));
    }
    let name = if cfg!(windows) {
        "flowgram-server.exe"
    } else {
        "flowgram-server"
    };
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join(name));
    }
    if let Ok(resources) = app.path().resource_dir() {
        candidates.push(resources.join(name));
    }
    if let Some(found) = candidates.iter().find(|candidate| candidate.is_file()) {
        return Ok(found.clone());
    }
    Err(format!(
        "FlowGram backend executable {name} was not found; set FLOWGRAM_SERVER_BIN or install the complete bundle (searched: {})",
        candidates
            .iter()
            .map(|candidate| candidate.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
    .into())
}

fn start_backend(app: &tauri::App) -> Result<Backend, Box<dyn std::error::Error>> {
    let dir = std::env::var_os("FLOWGRAM_DATA_DIR")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| app.path().app_data_dir())?;
    std::fs::create_dir_all(&dir)?;
    let executable = resolve_backend(app)?;
    let connection_file = dir.join("connection.json");
    if connection_file.exists() {
        std::fs::remove_file(&connection_file)?;
    }
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("backend.log"))?;
    let mut command = Command::new(executable);
    command
        .env("FLOWGRAM_DATA_DIR", &dir)
        .env("FLOWGRAM_PORT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log));
    let mut child = OwnedProcess::spawn(&mut command)?;
    // Wait only for metadata written by this owned process.
    for _ in 0..100 {
        if let Some(status) = child.try_wait()? {
            return Err(format!("FlowGram backend exited: {status}").into());
        }
        if let Ok(bytes) = std::fs::read(&connection_file)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && let (Some(endpoint), Some(token)) =
                (value["endpoint"].as_str(), value["token"].as_str())
        {
            let address = endpoint.trim_start_matches("http://");
            if std::net::TcpStream::connect_timeout(&address.parse()?, Duration::from_millis(100))
                .is_ok()
            {
                return Ok(Backend {
                    child: Mutex::new(Some(child)),
                    connection: Connection {
                        endpoint: endpoint.into(),
                        token: token.into(),
                    },
                });
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("FlowGram backend did not become ready".into())
}

#[cfg(target_os = "linux")]
fn apply_linux_webview_compat() {
    // WebKitGTK's DMA-BUF renderer draws an empty surface on some NVIDIA and
    // remote-desktop setups. Keep hardware rendering by default and let users
    // opt into the compatible path with FLOWGRAM_WEBKIT_COMPAT=1.
    if std::env::var_os("FLOWGRAM_WEBKIT_COMPAT").is_some_and(|value| value != "0") {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
}

#[cfg(target_os = "linux")]
fn apply_linux_backend_override() {
    // Some compositors deliver no pointer events to WebKitGTK on Wayland, while
    // others break under XWayland. Let users pick the GTK backend explicitly.
    if let Ok(backend) = std::env::var("FLOWGRAM_GDK_BACKEND")
        && matches!(backend.as_str(), "x11" | "wayland")
    {
        unsafe { std::env::set_var("GDK_BACKEND", backend) };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        apply_linux_backend_override();
        apply_linux_webview_compat();
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main")
                && let Ok(Some(monitor)) = window.current_monitor()
                && let Ok(size) = window.outer_size()
                && (size.width > monitor.work_area().size.width
                    || size.height > monitor.work_area().size.height)
            {
                // High-DPI or small displays must keep the footer and window controls reachable.
                window.maximize()?;
            }
            let backend = start_backend(app)?;
            app.manage(backend);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![backend_connection])
        .build(tauri::generate_context!())
        .expect("Unable to start FlowGram")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let backend = app.state::<Backend>();
                if let Ok(mut child) = backend.child.lock()
                    && let Some(process) = child.take()
                {
                    drop(process);
                }
            }
        });
}
