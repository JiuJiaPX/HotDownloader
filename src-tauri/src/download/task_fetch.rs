use std::time::Duration;

use tauri::AppHandle;

use crate::commands::api::download; // 获取下载链接

/// 重试获取下载链接（网络错误时最多尝试 3 次）
/// 传入 AppHandle，使下载链接获取函数能够读取登录态
pub(crate) async fn fetch_download_link_with_retry(
    app_handle: &AppHandle,
    song_mid: &str,
    filename: &str,
    task_id: &str,
) -> Result<(String, String), String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        match download::get_download_link(app_handle, song_mid, filename).await {
            Ok(link) => return Ok(link),
            Err(e) => {
                last_err = e;
                log::warn!(
                    "任务 {} 获取下载链接失败 (尝试 {}/3): {}",
                    task_id,
                    attempt + 1,
                    last_err
                );
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    // 1s, 2s, 4s
                }
            }
        }
    }
    Err(last_err)
}

/// 判断错误是否属于可重试的网络类错误
pub(crate) fn is_retryable_network_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || (err.is_request() && !err.is_body())
}
