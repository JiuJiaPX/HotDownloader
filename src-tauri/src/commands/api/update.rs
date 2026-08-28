use serde_json::{json, Value};
use tauri::command;

use super::client::CLIENT;

/// 检查 GitHub 最新发布版本
/// 通过 GitHub REST API 获取仓库最新 release 信息，返回 JSON 字符串
/// 字段包含：tag_name、name、body（更新内容）、html_url、published_at、prerelease、current_version、assets
#[command]
pub async fn check_update() -> Result<String, String> {
    let url = "https://api.github.com/repos/lerdb/HotDownloader/releases/latest";

    // 使用全局 CLIENT 发起 GET 请求，并携带 GitHub API 推荐的头信息
    let resp = CLIENT
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    // 提取关键字段（tag_name 可能带前缀 v，当前版本由 Cargo.toml 编译时提供）
    let tag_name = data["tag_name"].as_str().unwrap_or("").to_string();
    let name = data["name"].as_str().unwrap_or("").to_string();
    let body = data["body"].as_str().unwrap_or("").to_string();
    let html_url = data["html_url"].as_str().unwrap_or("").to_string();
    let published_at = data["published_at"].as_str().unwrap_or("").to_string();
    let prerelease = data["prerelease"].as_bool().unwrap_or(false);

    // 提取 assets 数组（发布资源），为前端提供下载安装包的直链信息
    let assets = data["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|asset| {
                    let name = asset["name"].as_str().unwrap_or("").to_string();
                    let browser_download_url = asset["browser_download_url"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let size = asset["size"].as_u64().unwrap_or(0);
                    if name.is_empty() || browser_download_url.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "name": name,
                        "browser_download_url": browser_download_url,
                        "size": size
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let result = json!({
        "tag_name": tag_name,
        "name": name,
        "body": body,
        "html_url": html_url,
        "published_at": published_at,
        "prerelease": prerelease,
        "current_version": env!("CARGO_PKG_VERSION"),
        "assets": assets
    });

    serde_json::to_string(&result).map_err(|e| format!("序列化结果失败: {}", e))
}
