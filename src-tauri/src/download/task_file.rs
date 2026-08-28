use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Seek};
use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_android_fs::{AndroidFsExt, FileAccessMode, FsUri};

use super::progress;

/// 文件写入缓冲区容量（64 KB）
const FILE_BUFFER_CAPACITY: usize = 64 * 1024;

/// 打开或创建下载目标文件，并返回 BufWriter。
/// - `downloaded`: 当前已下载字节数，函数内部可能将其重置为 0（如果文件异常）。
/// - `saf_file_uri`: 用于 SAF 模式，函数内部可能更新为实际文件的 URI。
/// 返回 `Some(BufWriter)` 表示成功；返回 `None` 表示发生错误（已发送错误事件），调用方应跳出下载循环。
pub(crate) async fn open_download_file(
    app_handle: &AppHandle,
    task_id: &str,
    download_dir: &str,
    is_saf: bool,
    saf_folder_uri: Option<&str>,
    downloaded: &mut u64,
    saf_file_uri: &mut Option<String>,
) -> Option<BufWriter<fs::File>> {
    if is_saf {
        open_saf_file(
            app_handle,
            task_id,
            download_dir,
            saf_folder_uri,
            downloaded,
            saf_file_uri,
        )
        .await
    } else {
        open_normal_file(app_handle, task_id, download_dir, downloaded)
    }
}

/// SAF 模式文件打开/创建
async fn open_saf_file(
    app_handle: &AppHandle,
    task_id: &str,
    download_dir: &str,
    saf_folder_uri: Option<&str>,
    downloaded: &mut u64,
    saf_file_uri: &mut Option<String>,
) -> Option<BufWriter<fs::File>> {
    let api = app_handle.android_fs();

    // 解析父目录 FsUri（包含 document_top_tree_uri）
    let parent_uri = match FsUri::from_json_str(saf_folder_uri?) {
        Ok(uri) => uri,
        Err(e) => {
            log::error!("解析 SAF 文件夹 URI 失败: {}", e);
            progress::emit_error(app_handle, task_id, "SAF 配置错误");
            return None;
        }
    };

    // download_dir 此时是文件名
    let file_path = Path::new(download_dir);

    // 尝试解析已存在的文件
    let existing_file_uri = api.resolve_file_uri(&parent_uri, file_path).ok();

    match existing_file_uri {
        Some(file_uri) => {
            // 文件已存在，记录最终文件 URI
            *saf_file_uri = Some(file_uri.uri.clone());

            if *downloaded > 0 {
                // 续传模式：打开文件并校验大小后 seek 到偏移量
                match api.open_file(&file_uri, FileAccessMode::ReadWrite) {
                    Ok(mut f) => {
                        // 校验文件大小：如果文件长度小于期望的偏移，说明文件异常，重置下载
                        let should_reset = match f.metadata() {
                            Ok(meta) => meta.len() < *downloaded,
                            Err(_) => true, // 无法获取元数据，保守重置
                        };

                        if should_reset {
                            log::warn!(
                                "任务 {} SAF 文件大小异常，重置下载（期望偏移 {}，实际大小 {}）",
                                task_id,
                                *downloaded,
                                f.metadata().map(|m| m.len()).unwrap_or(0)
                            );
                            // 清空文件并从头下载
                            if let Err(e) = f.set_len(0) {
                                log::error!("SAF 文件截断失败: {}", e);
                                progress::emit_error(app_handle, task_id, "文件异常，请重试");
                                return None;
                            }
                            if let Err(e) = f.seek(std::io::SeekFrom::Start(0)) {
                                log::error!("SAF 文件 seek 失败: {}", e);
                                progress::emit_error(app_handle, task_id, "文件定位失败");
                                return None;
                            }
                            *downloaded = 0;
                        } else {
                            // 文件大小正常，seek 到续传位置
                            if let Err(e) = f.seek(std::io::SeekFrom::Start(*downloaded)) {
                                log::error!("SAF 文件 seek 失败: {}", e);
                                progress::emit_error(app_handle, task_id, "文件定位失败");
                                return None;
                            }
                        }
                        Some(f)
                    }
                    Err(e) => {
                        log::error!("SAF 打开文件失败: {}", e);
                        progress::emit_error(app_handle, task_id, "无法打开文件");
                        None
                    }
                }
            } else {
                // 从头开始：截断文件
                match api.open_file_writable(&file_uri) {
                    Ok(f) => Some(f),
                    Err(e) => {
                        log::error!("SAF 打开文件失败: {}", e);
                        progress::emit_error(app_handle, task_id, "无法打开文件");
                        None
                    }
                }
            }
        }
        None => {
            // 文件不存在，创建新文件
            match api.create_new_file(&parent_uri, file_path, None) {
                Ok(file_uri) => {
                    *saf_file_uri = Some(file_uri.uri.clone());
                    // 新文件，重置偏移量
                    if *downloaded > 0 {
                        *downloaded = 0;
                    }
                    match api.open_file_writable(&file_uri) {
                        Ok(f) => Some(f),
                        Err(e) => {
                            log::error!("SAF 打开新文件失败: {}", e);
                            progress::emit_error(app_handle, task_id, "无法打开文件");
                            None
                        }
                    }
                }
                Err(e) => {
                    log::error!("SAF 创建文件失败: {}", e);
                    progress::emit_error(app_handle, task_id, "无法创建文件");
                    None
                }
            }
        }
    }
    .map(|f| BufWriter::with_capacity(FILE_BUFFER_CAPACITY, f))
}

/// 普通模式文件打开/创建
fn open_normal_file(
    app_handle: &AppHandle,
    task_id: &str,
    download_dir: &str,
    downloaded: &mut u64,
) -> Option<BufWriter<fs::File>> {
    if *downloaded == 0 {
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(download_dir)
        {
            Ok(f) => Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, f)),
            Err(e) => {
                log::error!("文件创建失败: {}", e);
                progress::emit_error(app_handle, task_id, "文件创建失败，请检查磁盘空间");
                None
            }
        }
    } else {
        // 续传任务，先以追加模式打开
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(download_dir)
        {
            Ok(f) => {
                // 校验文件大小：如果文件长度小于期望的偏移，说明文件异常，重置下载
                if let Ok(meta) = f.metadata() {
                    if meta.len() < *downloaded {
                        // 文件被截断或损坏，清空文件并从头下载
                        drop(f); // 先关闭文件，避免占用
                        match OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(download_dir)
                        {
                            Ok(new_f) => {
                                *downloaded = 0;
                                Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, new_f))
                            }
                            Err(e) => {
                                log::error!("文件重置失败: {}", e);
                                progress::emit_error(app_handle, task_id, "文件异常，请重试");
                                None
                            }
                        }
                    } else {
                        Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, f))
                    }
                } else {
                    // 无法获取元数据，保守起见改为从头下载
                    drop(f);
                    match OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(download_dir)
                    {
                        Ok(new_f) => {
                            *downloaded = 0;
                            Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, new_f))
                        }
                        Err(e) => {
                            log::error!("文件重置失败: {}", e);
                            progress::emit_error(app_handle, task_id, "文件异常，请重试");
                            None
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("文件打开失败: {}", e);
                progress::emit_error(app_handle, task_id, "文件访问失败");
                None
            }
        }
    }
}
