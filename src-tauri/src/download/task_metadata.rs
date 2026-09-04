use std::io::{Read, Seek, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::Duration;

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::tag::{Accessor, ItemKey, Tag, TagType};
use tauri::AppHandle;
use tauri_plugin_android_fs::{AndroidFsExt, FileAccessMode, FsUri};

use super::progress;
use crate::commands::api::client::CLIENT; // 全局 HTTP 客户端，用于下载封面
use crate::commands::api::lyrics::LyricResponse;

const METADATA_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// 将歌词与封面写入音频文件 metadata。
/// 普通模式直接操作文件路径；SAF 模式通过临时文件回写实现跨平台支持。
/// 错误或超时时通过 progress::emit_metadata_error 发送提示，不阻断下载完成。
pub(crate) async fn write_metadata(
    app_handle: &AppHandle,
    task_id: &str,
    file_path: &str,
    is_saf: bool,
    saf_file_uri: Option<String>,
    cover_url: &str,
    lyric: Option<LyricResponse>,
    track: u32,
    disc: u32,
    track_total: u32,
) {
    // 1. 从已获取的歌词响应中提取歌词内容：优先逐字歌词（elrc），其次普通歌词（lrc）
    let lyric_text = lyric.and_then(|resp| {
        if let Some(elrc) = resp.elrc.filter(|s| !s.trim().is_empty()) {
            Some(elrc)
        } else {
            resp.lrc.filter(|s| !s.trim().is_empty())
        }
    });

    // 2. 下载封面图片字节
    let cover_bytes = if !cover_url.is_empty() {
        match CLIENT.get(cover_url).send().await {
            Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
            _ => None,
        }
    } else {
        None
    };

    if lyric_text.is_none() && cover_bytes.is_none() && track == 0 {
        log::info!("无可用歌词、封面或曲序，跳过 metadata 写入");
        return;
    }

    let app = app_handle.clone();
    let task_id_owned = task_id.to_string();
    let file_path_owned = file_path.to_string();

    // lofty 读写是阻塞的；放进 blocking 线程，避免占满异步运行时把任务卡在「处理中」。
    let work = tokio::task::spawn_blocking(move || {
        let panic_result = catch_unwind(AssertUnwindSafe(|| {
            write_metadata_sync(
                &app,
                &task_id_owned,
                &file_path_owned,
                is_saf,
                saf_file_uri,
                lyric_text,
                cover_bytes,
                track,
                disc,
                track_total,
            );
        }));
        if panic_result.is_err() {
            log::error!("任务 {} 写入 metadata 时发生 panic，已跳过", task_id_owned);
            progress::emit_metadata_error(&app, &task_id_owned, "写入标签异常，已跳过");
        }
    });

    match tokio::time::timeout(METADATA_WRITE_TIMEOUT, work).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            log::warn!("任务 {} metadata 写入线程失败: {}", task_id, e);
            progress::emit_metadata_error(app_handle, task_id, "写入标签失败，已跳过");
        }
        Err(_) => {
            log::warn!("任务 {} metadata 写入超时，跳过以免卡住处理中", task_id);
            progress::emit_metadata_error(app_handle, task_id, "写入标签超时，已跳过");
        }
    }
}

fn write_metadata_sync(
    app_handle: &AppHandle,
    task_id: &str,
    file_path: &str,
    is_saf: bool,
    saf_file_uri: Option<String>,
    lyric_text: Option<String>,
    cover_bytes: Option<Vec<u8>>,
    track: u32,
    disc: u32,
    track_total: u32,
) {
    // 3. 准备本地临时路径：SAF 需先复制到临时文件
    let temp_path = if is_saf {
        let uri = match &saf_file_uri {
            Some(u) => u.clone(),
            None => {
                log::warn!("SAF 文件 URI 缺失，无法写入 metadata");
                progress::emit_metadata_error(app_handle, task_id, "SAF 文件 URI 缺失");
                return;
            }
        };
        let fs_uri = FsUri::from_uri(uri);
        let api = app_handle.android_fs();

        // 读取 SAF 文件并写入临时文件
        let mut src = match api.open_file(&fs_uri, FileAccessMode::Read) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("打开 SAF 文件读取失败: {}", e);
                progress::emit_metadata_error(
                    app_handle,
                    task_id,
                    &format!("打开 SAF 文件读取失败: {}", e),
                );
                return;
            }
        };
        let mut buf = Vec::new();
        if let Err(e) = src.read_to_end(&mut buf) {
            log::warn!("读取 SAF 文件失败: {}", e);
            progress::emit_metadata_error(
                app_handle,
                task_id,
                &format!("读取 SAF 文件失败: {}", e),
            );
            return;
        }
        // 从原始文件名提取扩展名，保证临时文件能被 lofty 正确识别格式
        let ext = Path::new(file_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("tmp");
        let temp = std::env::temp_dir().join(format!(
            "{}.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            ext
        ));
        if let Err(e) = std::fs::write(&temp, &buf) {
            log::warn!("写入临时文件失败: {}", e);
            progress::emit_metadata_error(app_handle, task_id, &format!("写入临时文件失败: {}", e));
            return;
        }
        temp
    } else {
        PathBuf::from(file_path)
    };

    // 4. 修改 metadata
    let mut tagged_file = match lofty::read_from_path(&temp_path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("读取音频文件失败: {}", e);
            if is_saf {
                let _ = std::fs::remove_file(&temp_path);
            }
            progress::emit_metadata_error(app_handle, task_id, &format!("读取音频文件失败: {}", e));
            return;
        }
    };

    // 始终移除 ID3v1。lofty 在把非 Latin-1 文本（中文歌词/曲名）写入 ID3v1 时会 panic；
    // release 若再配合 panic=abort，整个应用会直接闪退。
    if tagged_file.remove(TagType::Id3v1).is_some() {
        log::info!("已移除 ID3v1 标签，避免写入时 panic");
    }

    // 确保存在主标签
    let tag_type = tagged_file.primary_tag_type();
    if tagged_file.primary_tag().is_none() {
        let new_tag = Tag::new(tag_type);
        tagged_file.insert_tag(new_tag);
    }

    let tag = match tagged_file.primary_tag_mut() {
        Some(t) => t,
        None => {
            log::warn!("无法获取音频标签，跳过写入");
            if is_saf {
                let _ = std::fs::remove_file(&temp_path);
            }
            progress::emit_metadata_error(app_handle, task_id, "无法获取音频标签");
            return;
        }
    };

    // 写入歌词
    if let Some(lyric) = lyric_text {
        tag.remove_key(&ItemKey::Lyrics);
        tag.insert_text(ItemKey::Lyrics, lyric.clone());
    }

    // 写入封面
    if let Some(bytes) = cover_bytes {
        let picture = Picture::new_unchecked(
            PictureType::CoverFront,
            Some(lofty::picture::MimeType::Jpeg),
            None,
            bytes,
        );
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(picture);
    }

    // 写入曲序，供资源管理器「#」列与播放器按专辑顺序排列
    if track > 0 {
        tag.set_track(track);
        if track_total > 0 {
            tag.set_track_total(track_total);
        }
    }
    if disc > 0 {
        tag.set_disk(disc);
    }

    // 保存 metadata
    if let Err(e) = tagged_file.save_to_path(&temp_path, WriteOptions::default()) {
        log::warn!("保存 metadata 失败: {}", e);
        if is_saf {
            let _ = std::fs::remove_file(&temp_path);
        }
        progress::emit_metadata_error(app_handle, task_id, &format!("保存 metadata 失败: {}", e));
        return;
    } else {
        log::info!("metadata 已写入: {}", temp_path.display());
    }

    // 5. SAF 模式：将临时文件写回原文件
    if is_saf {
        if let Some(uri) = saf_file_uri {
            let fs_uri = FsUri::from_uri(uri);
            let api = app_handle.android_fs();
            match api.open_file(&fs_uri, FileAccessMode::ReadWrite) {
                Ok(mut dst) => {
                    let data = match std::fs::read(&temp_path) {
                        Ok(d) => d,
                        Err(e) => {
                            log::warn!("读取临时文件失败: {}", e);
                            let _ = std::fs::remove_file(&temp_path);
                            progress::emit_metadata_error(
                                app_handle,
                                task_id,
                                &format!("读取临时文件失败: {}", e),
                            );
                            return;
                        }
                    };
                    if let Err(e) = dst.set_len(0) {
                        log::warn!("清空 SAF 文件失败: {}", e);
                        let _ = std::fs::remove_file(&temp_path);
                        progress::emit_metadata_error(
                            app_handle,
                            task_id,
                            &format!("清空 SAF 文件失败: {}", e),
                        );
                        return;
                    }
                    if let Err(e) = dst.seek(std::io::SeekFrom::Start(0)) {
                        log::warn!("SAF 文件 seek 失败: {}", e);
                        let _ = std::fs::remove_file(&temp_path);
                        progress::emit_metadata_error(
                            app_handle,
                            task_id,
                            &format!("SAF 文件 seek 失败: {}", e),
                        );
                        return;
                    }
                    if let Err(e) = dst.write_all(&data) {
                        log::warn!("写入 SAF 文件失败: {}", e);
                        let _ = std::fs::remove_file(&temp_path);
                        progress::emit_metadata_error(
                            app_handle,
                            task_id,
                            &format!("写入 SAF 文件失败: {}", e),
                        );
                        return;
                    }
                }
                Err(e) => {
                    log::warn!("打开 SAF 文件写入失败: {}", e);
                    let _ = std::fs::remove_file(&temp_path);
                    progress::emit_metadata_error(
                        app_handle,
                        task_id,
                        &format!("打开 SAF 文件写入失败: {}", e),
                    );
                    return;
                }
            }
        }
        let _ = std::fs::remove_file(&temp_path);
    }
}
