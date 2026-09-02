//! 专辑搜索与专辑歌曲获取模块。
//!
//! 对齐 MusicBot-Go 的 QQ 音乐实现：
//! - 搜索优先 `DoSearchForQQMusicDesktop`（`search_type = 2`），
//!   失败或空结果时回退 `client_search_cp?t=8`
//! - 曲目使用 `music.musichallAlbum.AlbumSongList` / `GetAlbumSongList`
//!
//! 歌曲解析复用 [`super::parser::parse_song`]。

use serde_json::{json, Value};
use tauri::command;

use super::client::CLIENT;
use super::parser::parse_song;

const MUSICU_URL: &str =
    "https://u.y.qq.com/cgi-bin/musicu.fcg?format=json&inCharset=utf8&outCharset=utf8";
const LEGACY_SEARCH_URL: &str = "https://c.y.qq.com/soso/fcgi-bin/client_search_cp";

fn first_str(item: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = item[*key]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
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

/// 从搜索结果中的单个专辑条目提取统一格式。
fn parse_album_item(item: &Value) -> Option<Value> {
    let mut mid = first_str(item, &["albumMID", "albumMid", "albummid", "mid"]);
    let id = first_u64(item, &["albumID", "albumid", "id"]);
    if mid.is_empty() && id == 0 {
        return None;
    }
    // 无 mid 时用数字 ID 占位，后续 GetAlbumSongList 可按 albumID 拉取
    if mid.is_empty() {
        mid = id.to_string();
    }

    let name = first_str(item, &["albumName", "albumname", "name", "title"]);
    let artist = collect_artist_names(item);
    let song_count = first_u64(item, &["song_count", "songnum", "songNum", "songCount"]);
    let publish_time = first_str(
        item,
        &[
            "publicTime",
            "public_time",
            "publish_time",
            "publishDate",
            "time_public",
        ],
    );

    let mut pic_mid = first_str(item, &["pmid"]);
    if pic_mid.is_empty() {
        pic_mid = item["photo"]["pic_mid"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
    }
    let cover_mid = if pic_mid.is_empty() {
        mid.clone()
    } else {
        pic_mid
    };
    let cover_url = if cover_mid.is_empty() {
        first_str(item, &["albumPic", "coverUrl"])
    } else {
        format!(
            "https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg",
            cover_mid
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

/// 从专辑条目中收集歌手名，兼容多种字段形态。
fn collect_artist_names(item: &Value) -> String {
    let from_array = |arr: &Vec<Value>| -> String {
        arr.iter()
            .filter_map(|s| s["name"].as_str().map(String::from))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    };

    if let Some(arr) = item["singer_list"].as_array() {
        let names = from_array(arr);
        if !names.is_empty() {
            return names;
        }
    }
    if let Some(arr) = item["singer"].as_array() {
        let names = from_array(arr);
        if !names.is_empty() {
            return names;
        }
    }
    let name = first_str(item, &["singerName", "singername"]);
    if !name.is_empty() {
        return name;
    }
    if let Some(name) = item["singer"].as_str().filter(|s| !s.is_empty()) {
        return name.to_string();
    }
    String::new()
}

fn parse_album_list(list: &[Value]) -> Vec<Value> {
    list.iter().filter_map(parse_album_item).collect()
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

/// 桌面端搜索：`DoSearchForQQMusicDesktop` + `search_type = 2`。
/// 专辑列表在 `req.data.body.album.list`。
async fn search_albums_desktop(
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
                "search_type": 2,
            },
        },
    });

    let data = post_musicu(&request_body).await?;
    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let req = &data["req"];
    let req_code = req["code"].as_i64().unwrap_or(-1);
    // 2001 表示无结果，按空列表处理
    if req_code != 0 && req_code != 2001 {
        return Err(format!("搜索错误: req.code={}", req_code));
    }

    let body = &req["data"]["body"];
    let list = body["album"]["list"]
        .as_array()
        .or_else(|| body["item_album"].as_array());
    let albums = match list {
        Some(items) => parse_album_list(items),
        None => Vec::new(),
    };

    let meta = &req["data"]["meta"];
    let has_more = album_has_more(meta, albums.len() as u32, page, limit);

    Ok((albums, has_more))
}

/// 旧版搜索：`client_search_cp?t=8`。
async fn search_albums_legacy(
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
            ("t", "8"),
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

    let album_node = &data["data"]["album"];
    let albums = match album_node["list"].as_array() {
        Some(items) => parse_album_list(items),
        None => Vec::new(),
    };

    let has_more = album_has_more(album_node, albums.len() as u32, page, limit);
    Ok((albums, has_more))
}

fn album_has_more(meta: &Value, got: u32, page: u32, limit: u32) -> bool {
    let nextpage = meta["nextpage"].as_i64().unwrap_or(-1);
    if nextpage != -1 {
        return nextpage > 0;
    }

    let total = first_u64(meta, &["totalnum", "totalNum", "total"]);
    if total > 0 {
        return (page as u64) * (limit as u64) < total;
    }

    got >= limit && got > 0
}

/// 搜索专辑，返回 JSON 字符串，包含 `albums` 与 `has_more`。
///
/// # 参数
/// - `keyword`: 搜索关键字。
/// - `page`: 页码（从 1 开始）。
/// - `limit`: 每页数量。
#[command]
pub async fn search_albums(keyword: String, page: u32, limit: u32) -> Result<String, String> {
    let keyword = keyword.trim().to_string();
    if keyword.is_empty() {
        return Err("搜索关键字不能为空".into());
    }
    let page = page.max(1);
    let limit = limit.max(1);

    let desktop = search_albums_desktop(&keyword, page, limit).await;
    let (albums, has_more) = match desktop {
        Ok((albums, has_more)) if !albums.is_empty() => (albums, has_more),
        desktop_result => match search_albums_legacy(&keyword, page, limit).await {
            Ok((albums, has_more)) if !albums.is_empty() => (albums, has_more),
            Ok(legacy_empty) => desktop_result.unwrap_or(legacy_empty),
            Err(_legacy_err) => desktop_result?,
        },
    };

    serde_json::to_string(&json!({
        "albums": albums,
        "has_more": has_more
    }))
    .map_err(|e| format!("序列化结果失败: {}", e))
}

fn fallback_album_from_songs(album_mid: &str, album_id: u64, songs: &[Value], raw: Option<&Value>) -> Value {
    let first = songs.first();
    let name = first
        .and_then(|s| s["album"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            raw.map(|v| {
                let nested = first_str(&v["album"], &["name", "title"]);
                if nested.is_empty() {
                    first_str(v, &["albumname", "albumName"])
                } else {
                    nested
                }
            })
        })
        .unwrap_or_default();
    let artist = first
        .and_then(|s| s["artist"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| raw.map(collect_artist_names))
        .unwrap_or_default();
    let cover_url = first
        .and_then(|s| s["coverUrl"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if album_mid.is_empty() {
                String::new()
            } else {
                format!(
                    "https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg",
                    album_mid
                )
            }
        });
    let publish_time = raw
        .map(|v| {
            first_str(
                v,
                &["time_public", "pub_time", "publicTime", "publishDate"],
            )
        })
        .filter(|s| !s.is_empty())
        .or_else(|| {
            raw.and_then(|v| {
                let album = &v["album"];
                let t = first_str(
                    album,
                    &["time_public", "pub_time", "publicTime", "publishDate"],
                );
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            })
        })
        .unwrap_or_default();

    json!({
        "id": album_id,
        "mid": album_mid,
        "name": name,
        "artist": artist,
        "coverUrl": cover_url,
        "songCount": songs.len() as u64,
        "publishTime": publish_time,
    })
}

/// 获取专辑详情与曲目列表。
///
/// # 参数
/// - `album_mid`: 专辑 mid，或纯数字专辑 ID。
///
/// # 返回
/// JSON 字符串，包含 `album`（专辑信息）和 `songs`（歌曲列表）。
#[command]
pub async fn fetch_album_songs(album_mid: String) -> Result<String, String> {
    let album_mid = album_mid.trim().to_string();
    if album_mid.is_empty() {
        return Err("专辑 ID 不能为空".into());
    }

    let numeric_album_id: i64 = album_mid.parse().unwrap_or(0);

    let request_body = json!({
        "comm": {
            "ct": 24,
            "cv": 10000,
        },
        "albumSonglist": {
            "module": "music.musichallAlbum.AlbumSongList",
            "method": "GetAlbumSongList",
            "param": {
                "albumMid": album_mid,
                "albumID": numeric_album_id,
                "begin": 0,
                "num": 1000,
                "order": 2
            }
        }
    });

    let data = post_musicu(&request_body).await?;
    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }

    let songs_node = &data["albumSonglist"];
    if songs_node["code"] != 0 {
        return Err(format!(
            "获取专辑歌曲失败: code={}",
            songs_node["code"]
        ));
    }

    let song_list = songs_node["data"]["songList"]
        .as_array()
        .or_else(|| songs_node["data"]["songlist"].as_array())
        .ok_or("未找到专辑歌曲列表")?;

    let total_num = first_u64(&songs_node["data"], &["totalNum", "totalnum", "total"]);
    let resp_mid = first_str(&songs_node["data"], &["albumMid", "albumMID", "albummid"]);
    let album_mid_out = if resp_mid.is_empty() {
        album_mid.clone()
    } else {
        resp_mid
    };

    let mut songs = Vec::new();
    let mut first_raw: Option<Value> = None;
    for item in song_list {
        let nested = item.get("songInfo").or_else(|| item.get("songinfo"));
        let song_raw = match nested {
            Some(v) if v.is_object() => v,
            _ => item,
        };
        if first_raw.is_none() {
            first_raw = Some(song_raw.clone());
        }
        if let Some(mut song_obj) = parse_song(song_raw) {
            let fallback_track = (songs.len() as u32) + 1;
            if song_obj["track"].as_u64().unwrap_or(0) == 0 {
                song_obj["track"] = json!(fallback_track);
            }
            if total_num > 0 {
                song_obj["trackTotal"] = json!(total_num);
            }
            songs.push(song_obj);
        }
    }

    if songs.is_empty() && total_num == 0 && song_list.is_empty() {
        return Err("未找到专辑歌曲".into());
    }

    let mut album = fallback_album_from_songs(
        &album_mid_out,
        numeric_album_id.max(0) as u64,
        &songs,
        first_raw.as_ref(),
    );
    album["songCount"] = json!(if total_num > 0 {
        total_num
    } else {
        songs.len() as u64
    });
    album["mid"] = json!(album_mid_out);

    Ok(json!({
        "album": album,
        "songs": songs
    })
    .to_string())
}
