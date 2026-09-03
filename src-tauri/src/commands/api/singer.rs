//! 歌手搜索与歌手专辑列表。
//!
//! - 搜索优先 `DoSearchForQQMusicDesktop`（`search_type = 1`），
//!   失败或空结果时回退 `client_search_cp?t=9`
//! - 专辑列表使用 `music.musichallAlbum.AlbumListServer` / `GetAlbumList`

use serde_json::{json, Value};
use tauri::command;

use super::client::CLIENT;

const MUSICU_URL: &str =
    "https://u.y.qq.com/cgi-bin/musicu.fcg?format=json&inCharset=utf8&outCharset=utf8";
const LEGACY_SEARCH_URL: &str = "https://c.y.qq.com/soso/fcgi-bin/client_search_cp";
const LEGACY_SINGER_ALBUM_URL: &str = "https://c.y.qq.com/v8/fcg-bin/fcg_v8_singer_album.fcg";

fn first_str(item: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = item[*key].as_str().map(str::trim).filter(|s| !s.is_empty()) {
            return s.to_string();
        }
    }
    String::new()
}

fn first_u64(item: &Value, keys: &[&str]) -> u64 {
    for key in keys {
        let v = &item[*key];
        if let Some(n) = v.as_u64() {
            return n;
        }
        if let Some(n) = v.as_i64().filter(|n| *n >= 0).map(|n| n as u64) {
            return n;
        }
        if let Some(n) = v.as_str().and_then(|s| s.parse().ok()) {
            return n;
        }
    }
    0
}

fn parse_singer_item(item: &Value) -> Option<Value> {
    let mut mid = first_str(item, &["singerMID", "singerMid", "singermid", "mid"]);
    let id = first_u64(item, &["singerID", "singerId", "singerid", "id"]);
    if mid.is_empty() && id == 0 {
        return None;
    }
    if mid.is_empty() {
        mid = id.to_string();
    }

    let name = first_str(item, &["singerName", "singername", "name"]);
    let album_count = first_u64(item, &["albumNum", "albumnum", "albumCount"]);
    let song_count = first_u64(item, &["songNum", "songnum", "songCount"]);

    let mut cover_url = first_str(item, &["singerPic", "singer_pic", "pic", "coverUrl"]);
    if cover_url.is_empty() && !mid.is_empty() {
        cover_url = format!(
            "https://y.gtimg.cn/music/photo_new/T001R500x500M000{}.jpg",
            mid
        );
    }

    Some(json!({
        "id": id,
        "mid": mid,
        "name": name,
        "coverUrl": cover_url,
        "albumCount": album_count,
        "songCount": song_count,
    }))
}

fn parse_singer_list(list: &[Value]) -> Vec<Value> {
    list.iter().filter_map(parse_singer_item).collect()
}

fn parse_album_item(item: &Value, fallback_artist: &str) -> Option<Value> {
    let mut mid = first_str(item, &["albumMid", "albumMID", "albummid", "mid"]);
    let id = first_u64(item, &["albumID", "albumId", "albumid", "id"]);
    if mid.is_empty() && id == 0 {
        return None;
    }
    if mid.is_empty() {
        mid = id.to_string();
    }

    let name = first_str(item, &["albumName", "albumname", "name", "title"]);
    let mut artist = first_str(item, &["singerName", "singername"]);
    if artist.is_empty() {
        if let Some(arr) = item["singers"].as_array().or_else(|| item["singer"].as_array()) {
            artist = arr
                .iter()
                .filter_map(|s| s["name"].as_str().map(String::from))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
        }
    }
    if artist.is_empty() {
        artist = fallback_artist.to_string();
    }

    let song_count = first_u64(
        item,
        &["totalNum", "totalnum", "song_count", "songNum", "songCount"],
    );
    let publish_time = first_str(
        item,
        &[
            "publishDate",
            "pubTime",
            "publicTime",
            "public_time",
            "publish_time",
            "time_public",
        ],
    );

    let mut pic_mid = first_str(item, &["pmid", "pic_mid"]);
    if pic_mid.is_empty() {
        pic_mid = mid.clone();
    }
    let cover_url = if pic_mid.is_empty() {
        first_str(item, &["albumPic", "coverUrl"])
    } else {
        format!(
            "https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg",
            pic_mid
        )
    };

    Some(json!({
        "id": id,
        "mid": mid,
        "name": name,
        "artist": artist,
        "coverUrl": cover_url,
        "songCount": song_count,
        "publishTime": publish_time,
    }))
}

async fn post_musicu(body: &Value) -> Result<Value, String> {
    let resp = CLIENT
        .post(MUSICU_URL)
        .header("Content-Type", "application/json")
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", "https://y.qq.com")
        .header("Origin", "https://y.qq.com")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))
}

fn singer_has_more(meta: &Value, got: u32, page: u32, limit: u32) -> bool {
    let nextpage = meta["nextpage"].as_i64().unwrap_or(-1);
    if nextpage != -1 {
        return nextpage > 0;
    }

    let total = first_u64(meta, &["sum", "totalnum", "totalNum", "total"]);
    if total > 0 {
        return (page as u64) * (limit as u64) < total;
    }

    got >= limit && got > 0
}

async fn search_singers_desktop(
    keyword: &str,
    page: u32,
    limit: u32,
) -> Result<(Vec<Value>, bool), String> {
    let request_body = json!({
        "comm": {
            "ct": "19",
            "cv": "1859",
            "uin": "0",
        },
        "req": {
            "method": "DoSearchForQQMusicDesktop",
            "module": "music.search.SearchCgiService",
            "param": {
                "grp": 1,
                "num_per_page": limit,
                "page_num": page,
                "query": keyword,
                "search_type": 1,
            },
        },
    });

    let data = post_musicu(&request_body).await?;
    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let req = &data["req"];
    let req_code = req["code"].as_i64().unwrap_or(-1);
    if req_code != 0 && req_code != 2001 {
        return Err(format!("搜索错误: req.code={}", req_code));
    }

    let body = &req["data"]["body"];
    let list = body["singer"]["list"]
        .as_array()
        .or_else(|| body["item_singer"].as_array());
    let singers = match list {
        Some(items) => parse_singer_list(items),
        None => Vec::new(),
    };

    let meta = &req["data"]["meta"];
    let has_more = singer_has_more(meta, singers.len() as u32, page, limit);
    Ok((singers, has_more))
}

async fn search_singers_legacy(
    keyword: &str,
    page: u32,
    limit: u32,
) -> Result<(Vec<Value>, bool), String> {
    let resp = CLIENT
        .get(LEGACY_SEARCH_URL)
        .query(&[
            ("format", "json"),
            ("p", &page.to_string()),
            ("n", &limit.to_string()),
            ("w", keyword),
            ("t", "9"),
            ("aggr", "1"),
            ("cr", "1"),
            ("flag_qc", "0"),
        ])
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", "https://y.qq.com")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let singer_node = &data["data"]["singer"];
    let singers = match singer_node["list"].as_array() {
        Some(items) => parse_singer_list(items),
        None => Vec::new(),
    };

    let has_more = singer_has_more(singer_node, singers.len() as u32, page, limit);
    Ok((singers, has_more))
}

/// 搜索歌手，返回 JSON：`{ singers, has_more }`。
#[command]
pub async fn search_singers(keyword: String, page: u32, limit: u32) -> Result<String, String> {
    let keyword = keyword.trim().to_string();
    if keyword.is_empty() {
        return Err("搜索关键字不能为空".into());
    }
    let page = page.max(1);
    let limit = limit.max(1);

    let desktop = search_singers_desktop(&keyword, page, limit).await;
    let (singers, has_more) = match desktop {
        Ok((singers, has_more)) if !singers.is_empty() => (singers, has_more),
        desktop_result => match search_singers_legacy(&keyword, page, limit).await {
            Ok((singers, has_more)) if !singers.is_empty() => (singers, has_more),
            Ok(legacy_empty) => desktop_result.unwrap_or(legacy_empty),
            Err(_legacy_err) => desktop_result?,
        },
    };

    serde_json::to_string(&json!({
        "singers": singers,
        "has_more": has_more
    }))
    .map_err(|e| format!("序列化结果失败: {}", e))
}

async fn fetch_singer_albums_musicu(
    singer_mid: &str,
    begin: u32,
    num: u32,
) -> Result<(Vec<Value>, u64), String> {
    let request_body = json!({
        "comm": {
            "ct": 24,
            "cv": 0,
        },
        "singerAlbum": {
            "module": "music.musichallAlbum.AlbumListServer",
            "method": "GetAlbumList",
            "param": {
                "singerMid": singer_mid,
                "order": 0,
                "begin": begin,
                "num": num,
            }
        }
    });

    let data = post_musicu(&request_body).await?;
    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let node = &data["singerAlbum"];
    if node["code"] != 0 {
        return Err(format!("获取歌手专辑失败: code={}", node["code"]));
    }

    let album_data = &node["data"];
    let list = album_data["albumList"]
        .as_array()
        .or_else(|| album_data["list"].as_array())
        .ok_or("未找到歌手专辑列表")?;
    let total = first_u64(album_data, &["total", "totalNum", "totalnum"]);
    Ok((list.clone(), total))
}

async fn fetch_singer_albums_legacy(
    singer_mid: &str,
    begin: u32,
    num: u32,
) -> Result<(Vec<Value>, u64), String> {
    let resp = CLIENT
        .get(LEGACY_SINGER_ALBUM_URL)
        .query(&[
            ("format", "json"),
            ("outCharset", "utf-8"),
            ("singermid", singer_mid),
            ("order", "time"),
            ("begin", &begin.to_string()),
            ("num", &num.to_string()),
        ])
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", "https://y.qq.com")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let list = data["data"]["list"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let total = first_u64(&data["data"], &["total", "total_album", "totalNum"]);
    Ok((list, total))
}

/// 获取歌手专辑列表，返回 JSON：`{ albums, total, has_more }`。
///
/// # 参数
/// - `singer_mid`: 歌手 mid
/// - `begin`: 起始偏移（从 0 开始）
/// - `num`: 本次拉取数量
#[command]
pub async fn fetch_singer_albums(
    singer_mid: String,
    begin: u32,
    num: u32,
) -> Result<String, String> {
    let singer_mid = singer_mid.trim().to_string();
    if singer_mid.is_empty() {
        return Err("歌手 ID 不能为空".into());
    }
    let num = num.max(1);

    let musicu = fetch_singer_albums_musicu(&singer_mid, begin, num).await;
    let (raw_list, total) = match musicu {
        Ok((list, total)) if !list.is_empty() => (list, total),
        musicu_result => match fetch_singer_albums_legacy(&singer_mid, begin, num).await {
            Ok((list, total)) if !list.is_empty() => (list, total),
            Ok(legacy_empty) => musicu_result.unwrap_or(legacy_empty),
            Err(_legacy_err) => musicu_result?,
        },
    };

    let albums: Vec<Value> = raw_list
        .iter()
        .filter_map(|item| parse_album_item(item, ""))
        .collect();
    let got = albums.len() as u64;
    let has_more = if total > 0 {
        (begin as u64) + got < total
    } else {
        got >= num as u64 && got > 0
    };

    serde_json::to_string(&json!({
        "albums": albums,
        "total": total,
        "has_more": has_more,
    }))
    .map_err(|e| format!("序列化结果失败: {}", e))
}
