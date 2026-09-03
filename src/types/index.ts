// 所有品质标签，按从低到高排序
export const ALL_QUALITY_ORDER: string[] = [
    '48kacc',
    '96kacc',
    '192kacc',
    '96kogg',
    '192kogg',
    '128kmp3',
    '320kmp3',
    'ape',
    'flac',
    'hires',
    '杜比全景声',
    '臻品全景声',
    '臻品母带',
]

/** 降级顺序：从高到低 */
export const QUALITY_DOWNGRADE_ORDER: string[] = [...ALL_QUALITY_ORDER].reverse()

/**
 * 返回比目标音质更低的候选列表（从高到低）。
 * 自动降级只应选用更低音质，不能升到臻品母带等更高规格。
 */
export function getDowngradeCandidates(desiredQuality: string): string[] {
    const idx = ALL_QUALITY_ORDER.indexOf(desiredQuality)
    if (idx === -1) {
        return QUALITY_DOWNGRADE_ORDER
    }
    if (idx === 0) {
        return []
    }
    return ALL_QUALITY_ORDER.slice(0, idx).reverse()
}

export type Quality = string  // 不再限制字面量，兼容所有后端标签

export type TaskStatus = 'waiting' | 'downloading' | 'paused' | 'completed' | 'error' | 'processing'

export interface Settings {
    defaultQuality: Quality
    autoDowngrade: boolean
    downloadDir: string
    namingTemplate: string
    maxConcurrent: number
    jumpToTask: boolean
    // 新增 SAF 文件夹 URI 和名称
    safFolderUri?: string
    safFolderName?: string
    writeMetadata: boolean
    downloadLrc: boolean
    // 是否将歌曲保存到以专辑名命名的子文件夹
    downloadToAlbumFolder: boolean
    // 登录相关字段，可选，未登录时不设置
    loginUin?: string
    authst?: string
    refreshToken?: string
    refreshKey?: string
    accessToken?: string
    openid?: string
    // 重复文件处理策略：ask=弹窗询问，overwrite=覆盖，rename=自动重命名，cancel=取消下载
    duplicateStrategy?: 'ask' | 'overwrite' | 'rename' | 'cancel'
    // 下载完成后是否发送系统通知
    notifyOnComplete: boolean
}

/** 歌曲可用的单个品质项 */
export interface QualityItem {
    quality: string   // 品质标签，如 "128kmp3", "flac", "臻品母带" 等
    filename: string  // 对应下载文件名，如 "M800xxxx.mp3"
    size: number      // 文件字节大小
}

// 歌曲基本信息
export interface SongInfo {
    id: number
    mid: string
    title: string
    artist: string
    album: string
    coverUrl: string
    mediaMid: string
    qualities: QualityItem[]
    track?: number
    disc?: number
    trackTotal?: number
    /** 专辑歌手（用于专辑文件夹命名） */
    albumArtist?: string
    /** 专辑发布时间（用于专辑文件夹命名） */
    albumPublishTime?: string
    /** 专辑歌曲数（用于专辑文件夹命名） */
    albumSongCount?: number
}

// 搜索结果完整返回
export interface SearchResponse {
    songs: SongInfo[]
    has_more: boolean
}

// 歌单基本信息
export interface PlaylistInfo {
    id: string
    name: string
    creator: string
    coverUrl: string
    songCount: number
    playCount: number
}

// 歌单接口完整返回
export interface PlaylistSongsResponse {
    playlist: PlaylistInfo
    songs: SongInfo[]
}

// 专辑基本信息
export interface AlbumInfo {
    id: number
    mid: string
    name: string
    artist: string
    coverUrl: string
    songCount: number
    publishTime: string
}

// 专辑搜索结果
export interface AlbumSearchResponse {
    albums: AlbumInfo[]
    has_more: boolean
}

// 专辑详情与曲目
export interface AlbumSongsResponse {
    album: AlbumInfo
    songs: SongInfo[]
}

// 歌手基本信息
export interface SingerInfo {
    id: number
    mid: string
    name: string
    coverUrl: string
    albumCount: number
    songCount: number
}

// 歌手搜索结果
export interface SingerSearchResponse {
    singers: SingerInfo[]
    has_more: boolean
}

// 歌手专辑列表
export interface SingerAlbumsResponse {
    albums: AlbumInfo[]
    total: number
    has_more: boolean
}

// 搜索建议条目（对应后端 fetch_suggestions 返回的每个 item）
export interface SearchSuggestionItem {
    id?: string
    mid?: string
    name?: string
    singer?: string
    cover?: string | null
    vid?: string          // 仅 MV 类型存在
    [key: string]: unknown
}

// 搜索建议分组数据
export interface SearchSuggestionData {
    song: SearchSuggestionItem[]
    singer: SearchSuggestionItem[]
    album: SearchSuggestionItem[]
    mv: SearchSuggestionItem[]
}

// 歌词响应
export interface LyricResponse {
    lrc: string | null;
    elrc: string | null;
    raw: string | null;
    instrumental: boolean;
}

export interface TaskRecord {
    id: string
    songId: number
    songMid: string
    songTitle: string
    artist: string
    album: string
    coverUrl: string
    mediaMid: string           // 用于后续可能的操作
    filename: string           // 实际下载的品质文件名
    quality: Quality           // 实际选择的品质标签
    track?: number
    disc?: number
    trackTotal?: number
    status: TaskStatus
    errorMsg?: string
    filePath?: string
    fileSize: number
    downloaded: number
    retryCount: number
    addedAt: number
    speed?: number  // 实时下载速度 (bytes/s)，仅 downloading/paused 状态有意义
}

export interface DownloadProgressPayload {
    task_id: string
    downloaded: number
    total: number
    speed: number
}

export interface DownloadCompletedPayload {
    task_id: string
    final_path: string
    saf_folder_uri?: string | null
}

export interface DownloadErrorPayload {
    task_id: string
    error_msg: string
}

export interface DownloadLinkExpiredPayload {
    task_id: string
    current_offset: number
}

export const DEFAULT_SETTINGS: Settings = {
    defaultQuality: 'ask',
    autoDowngrade: true,
    downloadDir: '',
    namingTemplate: '{song} - {artist}',
    maxConcurrent: 3,
    jumpToTask: true,
    writeMetadata: false,
    downloadLrc: false,
    downloadToAlbumFolder: false,
    loginUin: '',
    authst: '',
    refreshToken: '',
    refreshKey: '',
    accessToken: '',
    openid: '',
    duplicateStrategy: 'ask',
    notifyOnComplete: false,
}

// GitHub 最新 release 信息
export interface UpdateInfo {
    tag_name: string
    name: string
    body: string
    html_url: string
    published_at: string
    prerelease: boolean
    current_version: string
    assets: UpdateAsset[]
}

// assets 字段：发布资源数组，用于展示下载安装包直链
export interface UpdateAsset {
    name: string
    browser_download_url: string
    size: number
}

// 文件下载完成、处理中事件载荷
export interface DownloadFileCompletePayload {
    task_id: string
}

// 元数据写入失败事件载荷
export interface DownloadMetadataErrorPayload {
    task_id: string
    error_msg: string
}