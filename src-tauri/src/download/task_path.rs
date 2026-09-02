use std::path::Path;

use tauri::AppHandle;

use super::task::SongInfo;
use crate::utils::filename;

/// 获取下载目录（绝对路径）及文件命名模板
pub(crate) async fn get_download_settings(
    app_handle: &AppHandle,
) -> (String, String, Option<String>, bool, bool, bool) {
    use crate::storage::store_wrapper;

    let default_dir = crate::commands::file_ops::get_default_download_dir_impl(app_handle);
    let default_template = "{song} - {artist}".to_string();

    let settings_json = store_wrapper::load_string(app_handle, "settings").unwrap_or_default();
    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).unwrap_or(serde_json::json!({}));

    let dir = settings
        .get("downloadDir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_dir.clone());

    // 过滤无效路径（Android 应用私有目录）
    let dir = if dir.contains("/data/user/0/") || dir.contains("/data/data/") {
        log::warn!("检测到应用私有目录路径，已回退为默认下载目录: {}", dir);
        default_dir
    } else if Path::new(&dir).is_absolute() || dir == "saf://" {
        dir
    } else {
        log::warn!(
            "下载目录不是绝对路径，已回退为默认下载目录: {}",
            default_dir
        );
        default_dir
    };

    let saf_folder_uri = settings
        .get("safFolderUri")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let template = settings
        .get("namingTemplate")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_template)
        .to_string();

    // 是否写入歌曲标签（歌词/封面）
    let write_metadata_enabled = settings
        .get("writeMetadata")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 是否单独下载 LRC 歌词文件
    let download_lrc_enabled = settings
        .get("downloadLrc")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 是否将歌曲保存到以专辑名命名的子文件夹
    let download_to_album_folder = settings
        .get("downloadToAlbumFolder")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    (
        dir,
        template,
        saf_folder_uri,
        write_metadata_enabled,
        download_lrc_enabled,
        download_to_album_folder,
    )
}

/// 整张专辑下载的目录覆盖。
/// - 桌面：本机音乐库 `Music/<productName>`，并强制专辑子文件夹
/// - Android：沿用用户已选的 SAF / 下载目录，只强制专辑子文件夹（不写公共 Music/）
pub(crate) fn apply_album_library_override(
    app: &AppHandle,
    use_music_library: bool,
    dir_setting: String,
    saf_uri_setting: Option<String>,
    download_to_album_folder: bool,
) -> (String, Option<String>, bool) {
    if !use_music_library {
        return (dir_setting, saf_uri_setting, download_to_album_folder);
    }
    #[cfg(target_os = "android")]
    {
        let _ = app;
        (dir_setting, saf_uri_setting, true)
    }
    #[cfg(not(target_os = "android"))]
    {
        (music_library_base_dir(app), None, true)
    }
}

/// 前端钉死的保存路径。Android 在 SAF 模式下传入的是相对路径（可含专辑子目录）。
pub(crate) fn resolve_pinned_save_path(
    save_path: &str,
    dir_setting: &str,
    saf_uri_setting: Option<&str>,
) -> (bool, String, Option<String>) {
    if cfg!(target_os = "android") && dir_setting == "saf://" && saf_uri_setting.is_some() {
        (
            true,
            save_path.replace('\\', "/"),
            saf_uri_setting.map(|s| s.to_string()),
        )
    } else {
        (false, save_path.to_string(), None)
    }
}

/// 本机音乐库下的软件目录：`Music/<productName>`（仅桌面端整张专辑下载使用）。
#[cfg(not(target_os = "android"))]
pub(crate) fn music_library_base_dir(app: &AppHandle) -> String {
    let product = app
        .config()
        .product_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("HotDownloader");

    let music_dir = dirs::audio_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Music")))
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    music_dir.join(product).to_string_lossy().to_string()
}

/// 将专辑名清洗为合法文件夹名；为空时回退为「未知专辑」。
fn album_folder_name(album: &str) -> String {
    let sanitized = filename::sanitize_name(album);
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "未知专辑".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 解析最终下载路径。
/// 根据传入的设置、模板和歌曲信息，返回是否为 SAF 模式、最终路径或文件名、SAF 文件夹 URI。
/// 该函数不负责读取设置，由调用方提供，避免重复调用 `get_download_settings`。
pub(crate) fn resolve_download_path(
    dir_setting: &str,
    template_setting: &str,
    saf_uri_setting: Option<&str>,
    song_info: &SongInfo,
    quality_filename: &str,
    download_to_album_folder: bool,
) -> (bool, String, Option<String>) {
    let fname = filename::build_filename(template_setting, song_info);
    let raw_ext = Path::new(quality_filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("flac");
    let ext = map_decrypted_extension(raw_ext);
    let file_name = format!("{}.{}", fname, ext);

    if dir_setting == "saf://" && cfg!(target_os = "android") && saf_uri_setting.is_some() {
        // SAF 相对路径必须使用 `/`，插件会按路径递归创建子目录
        let relative = if download_to_album_folder {
            format!("{}/{}", album_folder_name(&song_info.album), file_name)
        } else {
            file_name
        };
        (true, relative, saf_uri_setting.map(|s| s.to_string()))
    } else {
        let full_path = if download_to_album_folder {
            Path::new(dir_setting)
                .join(album_folder_name(&song_info.album))
                .join(file_name)
        } else {
            Path::new(dir_setting).join(file_name)
        };
        (false, full_path.to_string_lossy().to_string(), None)
    }
}

/// 将加密文件扩展名映射为解密后的真实扩展名
pub(crate) fn map_decrypted_extension(ext: &str) -> &str {
    match ext {
        "mgg" => "ogg",
        "mflac" => "flac",
        // 未知则保持原样
        _ => ext,
    }
}
