use once_cell::sync::Lazy;

/// 全局复用 HTTP 客户端，启用连接池、超时等
pub(crate) static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent("HotDownloader/1.0")
        .timeout(std::time::Duration::from_secs(30)) // 整体请求超时
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
});

/// 歌词接口专用 HTTP 客户端：仅设置超时，不设置 User-Agent（由请求时单独指定）
pub(crate) static LYRIC_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("Failed to build lyric HTTP client")
});
