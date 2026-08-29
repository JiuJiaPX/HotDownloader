use serde_json::{json, Value};
use tauri::command;

use super::client::CLIENT;
use super::parser::parse_song;
use crate::utils::guid::get_guid;

/// 搜索歌曲，返回 JSON 数组字符串（扩展 SongInfo，增加 mediaMid 和 qualities）
/// https://github.com/lyswhut/lx-music-desktop/blob/9c364b482e5621a1d38b50e8610d2fb974457e6e/src/renderer/utils/musicSdk/tx/musicSearch.js#L13
#[command]
pub async fn search_songs(keyword: String, page: u32, limit: u32) -> Result<String, String> {
    let searchid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();

    let request_body = json!({
        "comm": {
            "ct": "11",
            "cv": "14090508",
            "v": "14090508",
            "tmeAppID": "qqmusic",
            "guid": get_guid(),
            "phonetype": "EBG-AN10",
            "deviceScore": "553.47",
            "devicelevel": "50",
            "newdevicelevel": "20",
            "rom": "HuaWei/EMOTION/EmotionUI_14.2.0",
            "os_ver": "12",
            "OpenUDID": "0",
            "OpenUDID2": "0",
            "QIMEI36": "0",
            "udid": "0",
            "chid": "0",
            "aid": "0",
            "oaid": "0",
            "taid": "0",
            "tid": "0",
            "wid": "0",
            "uid": "0",
            "sid": "0",
            "modeSwitch": "6",
            "teenMode": "0",
            "ui_mode": "2",
            "nettype": "1020",
            "v4ip": ""
        },
        "req": {
            "module": "music.search.SearchCgiService",
            "method": "DoSearchForQQMusicMobile",
            "param": {
                "search_type": 0,
                "searchid": searchid,
                "query": keyword,
                "page_num": page,
                "num_per_page": limit,   // 使用参数控制每页数量
                "highlight": 0,
                "nqc_flag": 0,
                "multi_zhida": 0,
                "cat": 2,
                "grp": 1,
                "sin": 0,
                "sem": 0
            }
        }
    });

    // 发送 POST 请求
    let resp = CLIENT
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let data: Value = serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    // 检查整体状态
    if data["code"] != 0 {
        return Err(format!("接口错误: code={}", data["code"]));
    }
    let req = &data["req"];
    if req["code"] != 0 {
        return Err(format!("搜索错误: req.code={}", req["code"]));
    }

    // 提取歌曲列表
    let item_song = req["data"]["body"]["item_song"]
        .as_array()
        .ok_or("未找到歌曲列表")?;

    // 分页判断修改
    // 避免因 parse_song 过滤导致有效歌曲数不足一页时，误判为无更多结果，影响“加载更多”按钮显示。
    // 直接读取接口返回的 meta.nextpage 字段。该字段为 -1 表示无下一页，否则为下一页页码。
    let meta = &req["data"]["meta"];
    let nextpage = meta["nextpage"].as_i64().unwrap_or(-1);
    let has_more = nextpage != -1;

    let mut songs = Vec::new();
    for item in item_song {
        if let Some(song_obj) = parse_song(item) {
            songs.push(song_obj);
        }
    }

    // 返回包含歌曲列表和分页标志的 JSON 对象
    let result = json!({
        "songs": songs,
        "has_more": has_more
    });

    serde_json::to_string(&result).map_err(|e| format!("序列化结果失败: {}", e))
}
