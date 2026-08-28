use serde_json::{json, Value};
use tauri::command;
use url::Url;

use super::client::CLIENT;
use super::parser::parse_song;

/// 从用户输入中提取歌单 ID
fn extract_playlist_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("请输入歌单链接或 ID".into());
    }

    if input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(input.to_string());
    }

    let url = Url::parse(input).map_err(|_| "无法识别的歌单链接或 ID".to_string())?;

    if let Some((_, id)) = url.query_pairs().find(|(k, _)| k == "id") {
        let id = id.trim().to_string();
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
            return Ok(id);
        }
    }

    if let Some(segments) = url.path_segments() {
        let segs: Vec<&str> = segments.collect();
        if let Some(pos) = segs.iter().position(|s| *s == "playlist") {
            if let Some(id_part) = segs.get(pos + 1) {
                let id = id_part.trim_end_matches(".html");
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                    return Ok(id.to_string());
                }
            }
        }
    }

    Err("无法从链接中提取歌单 ID".into())
}

/// 获取歌单歌曲列表
/// https://github.com/lyswhut/lx-music-desktop/blob/9c364b482e5621a1d38b50e8610d2fb974457e6e/src/renderer/utils/musicSdk/tx/songList.js#L196
#[command]
pub async fn fetch_playlist_songs(input: String) -> Result<String, String> {
    let disstid = extract_playlist_id(&input)?;

    let base_url = "https://c.y.qq.com/qzone/fcg-bin/fcg_ucc_getcdinfo_byids_cp.fcg";
    let url = Url::parse_with_params(
        base_url,
        &[
            ("type", "1"),
            ("json", "1"),
            ("utf8", "1"),
            ("onlysong", "0"),
            ("new_format", "1"),
            ("disstid", disstid.as_str()),
            ("loginUin", "0"),
            ("hostUin", "0"),
            ("format", "json"),
            ("inCharset", "utf8"),
            ("outCharset", "utf-8"),
            ("notice", "0"),
            ("platform", "yqq.json"),
            ("needNewCode", "0"),
        ],
    )
    .map_err(|e| format!("URL 构建失败: {}", e))?;

    let resp = CLIENT
        .get(url)
        .header(
            "Referer",
            format!("https://y.qq.com/n/yqq/playsquare/{}.html", disstid),
        )
        .header("Origin", "https://y.qq.com")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("Accept", "*/*")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    let code = data["code"].as_i64().unwrap_or(-1);
    let subcode = data["subcode"].as_i64().unwrap_or(-1);
    if code != 0 || subcode != 0 {
        return Err(format!(
            "接口错误: code={}, subcode={}, msg={}",
            code,
            subcode,
            data["msg"].as_str().unwrap_or("")
        ));
    }

    let cd = data["cdlist"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("未找到歌单数据")?;

    let playlist = json!({
        "id": disstid,
        "name": cd["dissname"].as_str().unwrap_or(""),
        "creator": cd["nickname"].as_str().unwrap_or(""),
        "coverUrl": cd["logo"].as_str().unwrap_or(""),
        "songCount": cd["songnum"].as_u64().unwrap_or(0),
        "playCount": cd["visitnum"].as_u64().unwrap_or(0),
    });

    let songlist = cd["songlist"].as_array().ok_or("未找到歌曲列表")?;
    let mut songs = Vec::new();

    for song in songlist {
        if let Some(song_obj) = parse_song(song) {
            songs.push(song_obj);
        }
    }

    Ok(json!({
        "playlist": playlist,
        "songs": songs
    })
    .to_string())
}
