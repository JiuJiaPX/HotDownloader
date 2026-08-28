// parser.rs - 歌曲解析与品质构建
// 从原 api.rs 中提取，供搜索与歌单接口复用。

use serde_json::{json, Value};

/// 通用歌曲解析函数
/// - song: 歌曲原始 JSON 对象（搜索或歌单接口中的一项）
/// 标题优先使用 `name` 字段，若为空则使用 `title` 字段
/// 封面优先使用专辑 mid，其次歌手 mid，否则为空字符串
/// 返回 Option<Value>，当 mid 或 media_mid 为空时返回 None
pub(crate) fn parse_song(song: &Value) -> Option<Value> {
    // 歌曲唯一标识（使用 mid）
    let mid = song["mid"].as_str().unwrap_or("").to_string();
    if mid.is_empty() {
        return None;
    }

    // 数字歌曲 ID，用于歌词接口等需要数字 ID 的场景。
    // 兼容字段可能以数字或字符串形式出现。
    let song_id = song["id"]
        .as_u64()
        .or_else(|| song["id"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0);

    // 必须有 media_mid 才能下载，否则跳过
    let media_mid = song["file"]["media_mid"].as_str().unwrap_or("").to_string();
    if media_mid.is_empty() {
        return None;
    }

    // 标题：优先 name，若为空则 title
    let title = song["name"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            song["title"]
                .as_str()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_default();

    // 歌手列表，用逗号连接
    let singers: Vec<String> = song["singer"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let artist = singers.join(", ");

    // 专辑名
    let album_name = song["album"]["name"].as_str().unwrap_or("").to_string();

    // 封面：专辑 mid 优先，其次歌手 mid
    let album_mid = song["album"]["mid"].as_str().unwrap_or("");
    let first_singer_mid = song["singer"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|s| s["mid"].as_str())
        .unwrap_or("");

    // 封面URL，同样用可变变量避免 if 表达式问题
    let mut cover_url = String::new();
    if !album_mid.is_empty() && album_mid != "空" {
        cover_url = format!(
            "https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg",
            album_mid
        );
    } else if !first_singer_mid.is_empty() {
        cover_url = format!(
            "https://y.gtimg.cn/music/photo_new/T001R500x500M000{}.jpg",
            first_singer_mid
        );
    }

    // 品质列表（复用 build_qualities）
    let qualities = build_qualities(&song["file"], &song["vs"]);

    Some(json!({
        "id": song_id,
        "mid": mid,
        "title": title,
        "artist": artist,
        "album": album_name,
        "coverUrl": cover_url,
        "mediaMid": media_mid,
        "qualities": qualities
    }))
}

/// 根据 file 和 vs 生成可用品质列表
pub(crate) fn build_qualities(file: &Value, vs: &Value) -> Vec<Value> {
    let media_mid = file["media_mid"].as_str().unwrap_or("");
    let mut list = Vec::new();

    // 标准品质，按顺序定义 (前端标签, 文件前缀, 后缀, 文件大小字段名)
    let standard_qualities: Vec<(&str, &str, &str, &str)> = vec![
        ("48kacc", "C200", ".m4a", "size_48aac"),
        ("96kacc", "C400", ".m4a", "size_96aac"),
        ("192kacc", "C600", ".m4a", "size_192aac"),
        ("96kogg", "O4M0", ".mgg", "size_96ogg"),
        ("192kogg", "O6M0", ".mgg", "size_192ogg"),
        ("128kmp3", "M500", ".mp3", "size_128mp3"),
        ("320kmp3", "M800", ".mp3", "size_320mp3"),
        ("ape", "A000", ".ape", "size_ape"),
        ("flac", "F0M0", ".mflac", "size_flac"),
        ("hires", "RSM1", ".mflac", "size_hires"),
    ];

    for (label, prefix, suffix, size_key) in &standard_qualities {
        let size = file[*size_key].as_u64().unwrap_or(0);
        if size > 0 {
            list.push(json!({
                "quality": label,
                "filename": format!("{}{}{}", prefix, media_mid, suffix),
                "size": size
            }));
        }
    }

    // 特殊品质：杜比全景声 → 臻品全景声 → 臻品母带（按此顺序）
    let size_new = file["size_new"].as_array();
    let vs_arr = vs.as_array();
    if let (Some(size_new), Some(vs_arr)) = (size_new, vs_arr) {
        let vs3 = vs_arr.get(3).and_then(|v| v.as_str()).unwrap_or("");
        let vs4 = vs_arr.get(4).and_then(|v| v.as_str()).unwrap_or("");

        // 杜比全景声 (size_new[1] + vs[4])
        let size_dolby = size_new.get(1).and_then(|v| v.as_u64()).unwrap_or(0);
        if size_dolby > 0 && !vs4.is_empty() {
            list.push(json!({
                "quality": "杜比全景声",
                "filename": format!("Q0M0{}.mflac", vs4),
                "size": size_dolby
            }));
        }

        // 臻品全景声 (size_new[2] + vs[4])
        let size_panorama = size_new.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
        if size_panorama > 0 && !vs4.is_empty() {
            list.push(json!({
                "quality": "臻品全景声",
                "filename": format!("Q0M1{}.mflac", vs4),
                "size": size_panorama
            }));
        }

        // 臻品母带 (size_new[0] + vs[3])
        let size_master = size_new.first().and_then(|v| v.as_u64()).unwrap_or(0);
        if size_master > 0 && !vs3.is_empty() {
            list.push(json!({
                "quality": "臻品母带",
                "filename": format!("AIM0{}.mflac", vs3),
                "size": size_master
            }));
        }
    }

    list
}
