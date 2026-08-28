use serde_json::{json, Value};
use tauri::command;
use url::Url;

use super::client::{get_guid, CLIENT};

/// 获取热搜关键词列表，返回 JSON 数组字符串
/// https://github.com/lyswhut/lx-music-desktop/blob/9c364b482e5621a1d38b50e8610d2fb974457e6e/src/renderer/utils/musicSdk/tx/hotSearch.js#L15
#[command]
pub async fn fetch_hot_keywords() -> Result<String, String> {
    let request_body = json!({
        "comm": {
            "ct": "19",
            "cv": "1803",
            "guid": get_guid(),
            "patch": "118",
            "psrf_access_token_expiresAt": 0,
            "psrf_qqaccess_token": "",
            "psrf_qqopenid": "",
            "psrf_qqunionid": "",
            "tmeAppID": "qqmusic",
            "tmeLoginType": 0,
            "uin": "0",
            "wid": "0"
        },
        "hotkey": {
            "module": "tencent_musicsoso_hotkey.HotkeyService",
            "method": "GetHotkeyForQQMusicPC",
            "param": {
                "search_id": "",
                "uin": 0
            }
        }
    });

    let resp = CLIENT
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("Content-Type", "application/json")
        .header("Referer", "https://y.qq.com/portal/player.html")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    // 热搜数据在独立的 "hotkey" 字段中
    let hotkey = &data["hotkey"];
    if hotkey.is_null() {
        return Err("热搜数据缺失".into());
    }
    let code = hotkey["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        return Err(format!("热搜接口错误: code={}", code));
    }

    let vec_hotkey = hotkey["data"]["vec_hotkey"]
        .as_array()
        .ok_or("未找到热搜列表")?;

    let mut keywords = Vec::new();
    for item in vec_hotkey.iter().take(30) {
        if let Some(q) = item["query"].as_str() {
            if !q.is_empty() {
                keywords.push(q.to_string());
            }
        }
    }

    serde_json::to_string(&keywords).map_err(|e| format!("序列化结果失败: {}", e))
}

/// 获取搜索建议
/// https://github.com/lyswhut/lx-music-desktop/blob/9c364b482e5621a1d38b50e8610d2fb974457e6e/src/renderer/utils/musicSdk/tx/tipSearch.js#L10
#[command]
pub async fn fetch_suggestions(keyword: String) -> Result<String, String> {
    // 构建 URL，并进行 URL 编码
    let base_url = "https://c.y.qq.com/splcloud/fcgi-bin/smartbox_new.fcg";
    let url = Url::parse_with_params(
        base_url,
        &[
            ("is_xml", "0"),
            ("format", "json"),
            ("key", &keyword),
            ("loginUin", "0"),
            ("hostUin", "0"),
            ("inCharset", "utf8"),
            ("outCharset", "utf-8"),
            ("notice", "0"),
            ("platform", "yqq"),
            ("needNewCode", "0"),
        ],
    )
    .map_err(|e| format!("URL 构建失败: {}", e))?;

    let resp = CLIENT
        .get(url)
        .header("Referer", "https://y.qq.com/portal/player.html")
        .header("Accept", "*/*")
        .header("Host", "c.y.qq.com")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    // 检查状态码
    let code = data["code"].as_i64().unwrap_or(-1);
    let subcode = data["subcode"].as_i64().unwrap_or(-1);
    if code != 0 || subcode != 0 {
        return Err(format!("接口错误: code={}, subcode={}", code, subcode));
    }

    let root_data = data["data"].as_object().ok_or("缺少 data 字段")?;

    // 定义需要提取的类型列表及其对应的字段名
    let types = vec![
        ("song", "单曲"),
        ("singer", "歌手"),
        ("album", "专辑"),
        ("mv", "MV"),
    ];

    let mut result = serde_json::Map::new();

    for (type_key, _type_name) in types {
        let mut items = Vec::new();

        if let Some(obj) = root_data.get(type_key).and_then(|v| v.as_object()) {
            if let Some(itemlist) = obj.get("itemlist").and_then(|v| v.as_array()) {
                for item in itemlist {
                    let mut map = serde_json::Map::new();
                    // 通用字段
                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                        map.insert("id".to_string(), json!(id));
                    }
                    if let Some(mid) = item.get("mid").and_then(|v| v.as_str()) {
                        map.insert("mid".to_string(), json!(mid));
                    }
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        map.insert("name".to_string(), json!(name));
                    }
                    if let Some(singer) = item.get("singer").and_then(|v| v.as_str()) {
                        map.insert("singer".to_string(), json!(singer));
                    }
                    // 封面图片（歌手、专辑可能有，单曲通常没有）
                    if let Some(pic) = item.get("pic").and_then(|v| v.as_str()) {
                        map.insert("cover".to_string(), json!(pic));
                    } else {
                        map.insert("cover".to_string(), json!(null));
                    }
                    // MV 特有字段 vid
                    if type_key == "mv" {
                        if let Some(vid) = item.get("vid").and_then(|v| v.as_str()) {
                            map.insert("vid".to_string(), json!(vid));
                        }
                    }
                    items.push(Value::Object(map));
                }
            }
        }
        result.insert(type_key.to_string(), json!(items));
    }

    serde_json::to_string(&Value::Object(result)).map_err(|e| format!("序列化结果失败: {}", e))
}
