use std::fs::{self};
use std::io::{Seek, Write};
use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_android_fs::{AndroidFsExt, FileAccessMode, FsUri};

/// 将普通 LRC 歌词写入与歌曲同名的 `.lrc` 文件。
/// 支持普通模式与 SAF 模式，所有错误仅记录日志，不阻塞主下载流程。
pub(crate) async fn write_lrc_file(
    app_handle: &AppHandle,
    lrc_content: &str,
    song_file_path: &str,
    is_saf: bool,
    saf_folder_uri: Option<String>,
) -> Option<String> {
    // 提取歌曲文件名的 stem（不含扩展名）
    let song_name = Path::new(song_file_path);
    let stem = song_name
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    if !is_saf {
        // 普通模式：与歌曲文件同目录，生成 .lrc 文件
        let parent = song_name.parent().unwrap_or_else(|| Path::new("."));
        let lrc_path = parent.join(format!("{}.lrc", stem));
        if let Err(e) = fs::write(&lrc_path, lrc_content) {
            log::warn!("写入 LRC 歌词文件失败 {}: {}", lrc_path.display(), e);
            return None;
        } else {
            log::info!("LRC 歌词文件已保存: {}", lrc_path.display());
            return Some(lrc_path.to_string_lossy().to_string());
        }
    }

    // SAF 模式：在已授权的 SAF 目录中创建同名 .lrc 文件
    let parent_uri = match saf_folder_uri.as_deref() {
        Some(s) => match FsUri::from_json_str(s) {
            Ok(uri) => uri,
            Err(e) => {
                log::warn!("解析 SAF 文件夹 URI 失败: {}", e);
                return None;
            }
        },
        None => {
            log::warn!("SAF 文件夹 URI 缺失，无法创建 LRC 文件");
            return None;
        }
    };

    // SAF 相对路径可能包含专辑子目录，歌词文件需与歌曲放在同一文件夹
    let lrc_file_name = match song_name.parent() {
        Some(parent) if !parent.as_os_str().is_empty() && parent != Path::new(".") => parent
            .join(format!("{}.lrc", stem))
            .to_string_lossy()
            .replace('\\', "/"),
        _ => format!("{}.lrc", stem),
    };
    let api = app_handle.android_fs();

    // 尝试解析已存在的 LRC 文件，若存在则打开可写并清空；否则创建新文件
    let file_path = std::path::Path::new(&lrc_file_name);
    let file_uri_opt = api.resolve_file_uri(&parent_uri, file_path).ok();

    match file_uri_opt {
        Some(existing_uri) => {
            // 文件已存在：打开并清空后写入
            match api.open_file(&existing_uri, FileAccessMode::ReadWrite) {
                Ok(mut f) => {
                    if let Err(e) = f.set_len(0) {
                        log::warn!("清空 LRC 文件失败: {}", e);
                        return None;
                    }
                    if let Err(e) = f.seek(std::io::SeekFrom::Start(0)) {
                        log::warn!("LRC 文件 seek 失败: {}", e);
                        return None;
                    }
                    if let Err(e) = f.write_all(lrc_content.as_bytes()) {
                        log::warn!("写入 LRC 歌词内容失败: {}", e);
                        return None;
                    }
                    log::info!("SAF LRC 歌词文件已保存: {}", existing_uri.uri);
                    Some(existing_uri.uri.clone())
                }
                Err(e) => {
                    log::warn!("打开已有 LRC 文件失败: {}", e);
                    None
                }
            }
        }
        None => {
            // 文件不存在：创建新文件
            match api.create_new_file(&parent_uri, file_path, None) {
                Ok(new_uri) => match api.open_file_writable(&new_uri) {
                    Ok(mut f) => {
                        if let Err(e) = f.write_all(lrc_content.as_bytes()) {
                            log::warn!("写入 LRC 歌词内容失败: {}", e);
                            return None;
                        }
                        log::info!("SAF LRC 歌词文件已保存: {}", new_uri.uri);
                        Some(new_uri.uri.clone())
                    }
                    Err(e) => {
                        log::warn!("打开新 LRC 文件失败: {}", e);
                        None
                    }
                },
                Err(e) => {
                    log::warn!("创建 LRC 文件失败: {}", e);
                    None
                }
            }
        }
    }
}
