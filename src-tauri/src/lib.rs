use base64::engine::general_purpose;
use base64::Engine;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::command;
use tauri::Manager;

#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    ModifiedDesc,
    NameAsc,
}

struct AppState {
    current_source: Mutex<PathBuf>,
    source_list: Mutex<Vec<PathBuf>>,
    sort_mode: Mutex<SortMode>,
}

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
}

fn get_mime(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

fn build_source_list(source_path: &PathBuf, sort_mode: SortMode) -> Vec<PathBuf> {
    let dir = match source_path.parent() {
        Some(p) => p,
        None => return vec![],
    };
    let is_dir_mode = source_path.is_dir();

    let mut list: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    if is_dir_mode {
                        p.is_dir()
                    } else {
                        p.is_file()
                            && p.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| e.eq_ignore_ascii_case("zip"))
                                .unwrap_or(false)
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    match sort_mode {
        SortMode::ModifiedDesc => {
            list.sort_by_key(|p| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            list.reverse();
        }
        SortMode::NameAsc => {
            list.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        }
    }
    list
}

fn get_image_list(source: &PathBuf) -> Result<Vec<String>, String> {
    if source.is_dir() {
        let mut names: Vec<String> = std::fs::read_dir(source)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_image(&p.to_string_lossy()))
            .filter_map(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .collect();
        names.sort();
        Ok(names)
    } else {
        let file = std::fs::File::open(source).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                if !entry.is_dir() && is_image(entry.name()) {
                    names.push(entry.name().to_string());
                }
            }
        }
        Ok(names)
    }
}

#[command]
fn get_current_source(state: tauri::State<AppState>) -> String {
    state.current_source.lock().unwrap().to_string_lossy().to_string()
}

#[command]
fn get_image_count(state: tauri::State<AppState>) -> Result<usize, String> {
    let path = state.current_source.lock().unwrap().clone();
    get_image_list(&path).map(|v| v.len())
}

#[command]
fn load_image(
    index: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<String, String> {
    let path = state.current_source.lock().unwrap().clone();
    let names = get_image_list(&path)?;
    let total = names.len();
    let name = names.get(index).ok_or("인덱스 범위 초과")?;

    let bytes = if path.is_dir() {
        std::fs::read(path.join(name)).map_err(|e| e.to_string())?
    } else {
        let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut entry = archive.by_name(name).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        buf
    };

    let source_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let title = format!("{} — {} / {}", source_name, index + 1, total);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&title);
    }

    let mime = get_mime(name);
    let b64 = general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[command]
fn move_archive(direction: i32, state: tauri::State<AppState>) -> Result<String, String> {
    let current = state.current_source.lock().unwrap().clone();
    let list = state.source_list.lock().unwrap();
    let idx = list
        .iter()
        .position(|p| *p == current)
        .ok_or("현재 소스를 목록에서 찾을 수 없음")?;
    let next_idx = idx as i32 + direction;
    if next_idx < 0 || next_idx >= list.len() as i32 {
        return Err("인덱스 범위 초과".to_string());
    }
    let next = list[next_idx as usize].clone();
    drop(list);
    *state.current_source.lock().unwrap() = next.clone();
    Ok(next.to_string_lossy().to_string())
}

#[command]
fn toggle_sort(state: tauri::State<AppState>) -> Result<String, String> {
    let mut mode = state.sort_mode.lock().unwrap();
    *mode = match *mode {
        SortMode::ModifiedDesc => SortMode::NameAsc,
        SortMode::NameAsc => SortMode::ModifiedDesc,
    };
    let new_mode = *mode;
    drop(mode);

    let current = state.current_source.lock().unwrap().clone();
    let new_list = build_source_list(&current, new_mode);
    *state.source_list.lock().unwrap() = new_list;

    let label = match new_mode {
        SortMode::ModifiedDesc => "modified-desc",
        SortMode::NameAsc => "name-asc",
    };
    Ok(label.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let source_path = PathBuf::from(args.get(1).cloned().unwrap_or_default());
    let sort_mode = SortMode::ModifiedDesc;
    let source_list = build_source_list(&source_path, sort_mode);

    tauri::Builder::default()
        .setup(move |app| {
            app.manage(AppState {
                current_source: Mutex::new(source_path),
                source_list: Mutex::new(source_list),
                sort_mode: Mutex::new(sort_mode),
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_current_source,
            get_image_count,
            load_image,
            move_archive,
            toggle_sort,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
