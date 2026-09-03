<template>
    <div class="search-view">
        <div v-if="!albumDetail && !singerDetail" class="search-header">
            <n-radio-group :value="searchType" size="medium" class="search-type-group"
                @update:value="onSearchTypeChange">
                <n-radio-button value="song">搜索歌曲</n-radio-button>
                <n-radio-button value="album">搜索专辑</n-radio-button>
                <n-radio-button value="singer">搜索歌手</n-radio-button>
            </n-radio-group>
            <SearchBar v-model:keyword="keyword" :placeholder="searchPlaceholder" @search="handleSearch" />
        </div>

        <!-- 输入非空且未搜索：显示搜索建议 -->
        <SearchSuggestions v-if="showSuggestions" :data="suggestions" @select="onSuggestionSelect"
            @select-album="onSuggestionAlbumSelect" @select-singer="onSuggestionSingerSelect" />

        <!-- 输入为空且未搜索：显示历史与热搜 -->
        <div v-if="!keyword && !hasSearched && !albumDetail && !singerDetail">
            <SearchHistory :history="historyStore.history" @select="onHistorySelect" @remove="onHistoryRemove"
                @clear="historyStore.clearHistory" />
            <HotKeywords :keywords="hotKeywords" :loading="hotLoading" @select="onHotClick" />
        </div>

        <!-- 加载中 -->
        <div v-if="loading && !albumDetail && !singerDetail" class="loading-wrapper">
            <n-spin size="medium" />
        </div>

        <!-- 歌曲搜索结果 -->
        <SearchResultList v-if="hasSearched && !loading && searchType === 'song' && !albumDetail && !singerDetail"
            :songs="searchResults" v-model:selectedIds="selectedIds" :has-more="hasMore" :loading-more="loadingMore"
            @download="onSingleDownload" @retry="handleSearch" @load-more="loadMore" />

        <!-- 专辑搜索结果 -->
        <AlbumResultList v-if="hasSearched && !loading && searchType === 'album' && !albumDetail && !singerDetail"
            :albums="albumResults" :has-more="albumHasMore" :loading-more="albumLoadingMore" @open="openAlbum"
            @retry="handleSearch" @load-more="loadMoreAlbums" />

        <!-- 歌手搜索结果 -->
        <SingerResultList v-if="hasSearched && !loading && searchType === 'singer' && !albumDetail && !singerDetail"
            :singers="singerResults" :has-more="singerHasMore" :loading-more="singerLoadingMore" @open="openSinger"
            @retry="handleSearch" @load-more="loadMoreSingers" />

        <!-- 歌手详情（专辑列表） -->
        <SingerDetail v-if="singerDetail && !albumDetail" :singer="singerDetailInfo" :albums="singerAlbums"
            :total="singerAlbumsTotal" :loading="singerDetailLoading" :loading-more="singerAlbumsLoadingMore"
            :has-more="singerAlbumsHasMore" :error-msg="singerDetailError" :downloading-mid="downloadingAlbumMid"
            @back="closeSingerDetail" @open-album="openAlbum" @download-album="onDownloadSingerAlbum"
            @load-more="loadMoreSingerAlbums" />

        <!-- 专辑详情 -->
        <AlbumDetail v-if="albumDetail" :album="albumDetailInfo" :songs="albumSongs" :loading="albumDetailLoading"
            :error-msg="albumDetailError" v-model:selectedIds="albumSelectedIds" @back="closeAlbumDetail"
            @download="onSingleDownload" @batch-download="onAlbumBatchDownload"
            @download-album="onDownloadAlbum" />

        <BatchDownloadBar v-if="selectedIds.length > 0 && !albumDetail && !singerDetail"
            :selectedCount="selectedIds.length" @batch-download="onBatchDownload" />
    </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue'
import { NSpin, NRadioGroup, NRadioButton, useNotification } from 'naive-ui'
import SearchBar from '../components/search/SearchBar.vue'
import SearchHistory from '../components/search/SearchHistory.vue'
import HotKeywords from '../components/search/HotKeywords.vue'
import SearchSuggestions from '../components/search/SearchSuggestions.vue'
import SearchResultList from '../components/search/SearchResultList.vue'
import AlbumResultList from '../components/search/AlbumResultList.vue'
import AlbumDetail from '../components/search/AlbumDetail.vue'
import SingerResultList from '../components/search/SingerResultList.vue'
import SingerDetail from '../components/search/SingerDetail.vue'
import BatchDownloadBar from '../components/search/BatchDownloadBar.vue'
import { useHistoryStore } from '../stores/historyStore'
import { useDownloadActions } from '../composables/useDownloadActions'
import * as musicApi from '../api/musicApi'
import type {
    SongInfo,
    SearchSuggestionData,
    SearchSuggestionItem,
    AlbumInfo,
    SingerInfo,
} from '../types'

type SearchType = 'song' | 'album' | 'singer'

const keyword = ref('')
const searchType = ref<SearchType>('song')
const searchResults = ref<SongInfo[]>([])
const albumResults = ref<AlbumInfo[]>([])
const singerResults = ref<SingerInfo[]>([])
const selectedIds = ref<string[]>([])
const loading = ref(false)
const hasSearched = ref(false)

const PAGE_SIZE = 20
const currentPage = ref(1)
const hasMore = ref(false)
const loadingMore = ref(false)

const albumCurrentPage = ref(1)
const albumHasMore = ref(false)
const albumLoadingMore = ref(false)
const lastSongKeyword = ref('')
const lastAlbumKeyword = ref('')
const lastSingerKeyword = ref('')

const singerCurrentPage = ref(1)
const singerHasMore = ref(false)
const singerLoadingMore = ref(false)

const albumDetail = ref(false)
const albumDetailInfo = ref<AlbumInfo | null>(null)
const albumSongs = ref<SongInfo[]>([])
const albumDetailLoading = ref(false)
const albumDetailError = ref('')
const albumSelectedIds = ref<string[]>([])

const singerDetail = ref(false)
const singerDetailInfo = ref<SingerInfo | null>(null)
const singerAlbums = ref<AlbumInfo[]>([])
const singerAlbumsTotal = ref(0)
const singerAlbumsBegin = ref(0)
const singerAlbumsHasMore = ref(false)
const singerAlbumsLoadingMore = ref(false)
const singerDetailLoading = ref(false)
const singerDetailError = ref('')
const downloadingAlbumMid = ref('')
const albumOpenedFromSinger = ref(false)

const hotKeywords = ref<string[]>([])
const hotLoading = ref(false)

const historyStore = useHistoryStore()
const notification = useNotification()
const { downloadSingle, batchDownload } = useDownloadActions()

const suggestions = ref<SearchSuggestionData>({
    song: [],
    singer: [],
    album: [],
    mv: [],
})

let abortController: AbortController | null = null
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const showSuggestions = computed(() => {
    return keyword.value.trim() !== '' && !hasSearched.value && !albumDetail.value && !singerDetail.value
})

const searchPlaceholder = computed(() => {
    if (searchType.value === 'album') return '搜索专辑'
    if (searchType.value === 'singer') return '搜索歌手'
    return '搜索歌曲、歌手、专辑'
})

watch(keyword, (newVal) => {
    if (debounceTimer) {
        clearTimeout(debounceTimer)
    }
    if (abortController) {
        abortController.abort()
    }

    const term = newVal.trim()
    if (!term) {
        suggestions.value = { song: [], singer: [], album: [], mv: [] }
        return
    }

    debounceTimer = setTimeout(async () => {
        const controller = new AbortController()
        abortController = controller
        try {
            const res = await musicApi.fetchSuggestions(term)
            if (!controller.signal.aborted) {
                suggestions.value = res
            }
        } catch {
            if (!controller.signal.aborted) {
                suggestions.value = { song: [], singer: [], album: [], mv: [] }
            }
        } finally {
            if (abortController === controller) {
                abortController = null
            }
        }
    }, 300)
})

function onSuggestionSelect(word: string) {
    keyword.value = word
    searchType.value = 'song'
    handleSearch()
}

async function onSuggestionAlbumSelect(item: SearchSuggestionItem) {
    if (item.name) {
        keyword.value = item.name
    }
    const mid = item.mid || (typeof item.id === 'string' ? item.id : '')
    if (!mid) {
        searchType.value = 'album'
        handleSearch()
        return
    }
    historyStore.addHistory(item.name || mid)
    await openAlbumByMid(mid, item.name || '', item.singer || '')
}

async function onSuggestionSingerSelect(item: SearchSuggestionItem) {
    if (item.name) {
        keyword.value = item.name
    }
    const mid = item.mid || (typeof item.id === 'string' ? item.id : '')
    if (!mid) {
        searchType.value = 'singer'
        handleSearch()
        return
    }
    historyStore.addHistory(item.name || mid)
    await openSingerByMid(mid, item.name || '')
}

watch(keyword, (newVal) => {
    if (!newVal) {
        hasSearched.value = false
        searchResults.value = []
        albumResults.value = []
        singerResults.value = []
        selectedIds.value = []
        suggestions.value = { song: [], singer: [], album: [], mv: [] }
        currentPage.value = 1
        hasMore.value = false
        loadingMore.value = false
        albumCurrentPage.value = 1
        albumHasMore.value = false
        albumLoadingMore.value = false
        singerCurrentPage.value = 1
        singerHasMore.value = false
        singerLoadingMore.value = false
        lastSongKeyword.value = ''
        lastAlbumKeyword.value = ''
        lastSingerKeyword.value = ''
        resetAlbumDetail()
        resetSingerDetail()
    }
})

async function fetchHotKeywords() {
    hotLoading.value = true
    try {
        hotKeywords.value = await musicApi.getHotKeywords()
    } catch {
        hotKeywords.value = []
    } finally {
        hotLoading.value = false
    }
}

onMounted(() => {
    fetchHotKeywords()
})

function onHotClick(word: string) {
    keyword.value = word
    handleSearch()
}

function onHistorySelect(term: string) {
    keyword.value = term
    handleSearch()
}

function onHistoryRemove(term: string) {
    historyStore.removeHistoryItem(term)
}

function onSearchTypeChange(type: string) {
    if (type !== 'song' && type !== 'album' && type !== 'singer') return
    if (searchType.value === type) return
    searchType.value = type
    selectedIds.value = []
    const term = keyword.value.trim()
    if (!hasSearched.value || !term) return
    if (type === 'song' && lastSongKeyword.value === term) return
    if (type === 'album' && lastAlbumKeyword.value === term) return
    if (type === 'singer' && lastSingerKeyword.value === term) return
    handleSearch()
}

async function handleSearch() {
    const term = keyword.value.trim()
    if (!term) return

    resetAlbumDetail()
    resetSingerDetail()
    loading.value = true
    hasSearched.value = true
    selectedIds.value = []

    try {
        if (searchType.value === 'album') {
            albumCurrentPage.value = 1
            albumHasMore.value = false
            albumLoadingMore.value = false
            const response = await musicApi.searchAlbums(term, albumCurrentPage.value, PAGE_SIZE)
            albumResults.value = response.albums
            albumHasMore.value = response.has_more
            lastAlbumKeyword.value = term
        } else if (searchType.value === 'singer') {
            singerCurrentPage.value = 1
            singerHasMore.value = false
            singerLoadingMore.value = false
            const response = await musicApi.searchSingers(term, singerCurrentPage.value, PAGE_SIZE)
            singerResults.value = response.singers
            singerHasMore.value = response.has_more
            lastSingerKeyword.value = term
        } else {
            currentPage.value = 1
            hasMore.value = false
            loadingMore.value = false
            const response = await musicApi.searchSongs(term, currentPage.value, PAGE_SIZE)
            searchResults.value = response.songs
            hasMore.value = response.has_more
            lastSongKeyword.value = term
        }
        historyStore.addHistory(term)
    } catch (error) {
        console.error('搜索失败:', error)
        if (searchType.value === 'album') {
            albumResults.value = []
            albumHasMore.value = false
        } else if (searchType.value === 'singer') {
            singerResults.value = []
            singerHasMore.value = false
        } else {
            searchResults.value = []
            hasMore.value = false
        }
    } finally {
        loading.value = false
    }
}

async function loadMore() {
    if (loading.value || loadingMore.value || !hasMore.value) return

    const nextPage = currentPage.value + 1
    loadingMore.value = true

    try {
        const response = await musicApi.searchSongs(keyword.value.trim(), nextPage, PAGE_SIZE)
        const more = response.songs

        const existingIds = new Set(searchResults.value.map((s) => s.mid))
        const newSongs = more.filter((s) => !existingIds.has(s.mid))
        searchResults.value = [...searchResults.value, ...newSongs]

        currentPage.value = nextPage
        hasMore.value = response.has_more
    } catch (error) {
        console.error('加载更多失败:', error)
    } finally {
        loadingMore.value = false
    }
}

async function loadMoreAlbums() {
    if (loading.value || albumLoadingMore.value || !albumHasMore.value) return

    const nextPage = albumCurrentPage.value + 1
    albumLoadingMore.value = true

    try {
        const response = await musicApi.searchAlbums(keyword.value.trim(), nextPage, PAGE_SIZE)
        const existingIds = new Set(albumResults.value.map((a) => a.mid))
        const newAlbums = response.albums.filter((a) => !existingIds.has(a.mid))
        albumResults.value = [...albumResults.value, ...newAlbums]
        albumCurrentPage.value = nextPage
        albumHasMore.value = response.has_more
    } catch (error) {
        console.error('加载更多专辑失败:', error)
    } finally {
        albumLoadingMore.value = false
    }
}

async function loadMoreSingers() {
    if (loading.value || singerLoadingMore.value || !singerHasMore.value) return

    const nextPage = singerCurrentPage.value + 1
    singerLoadingMore.value = true

    try {
        const response = await musicApi.searchSingers(keyword.value.trim(), nextPage, PAGE_SIZE)
        const existingIds = new Set(singerResults.value.map((s) => s.mid))
        const newSingers = response.singers.filter((s) => !existingIds.has(s.mid))
        singerResults.value = [...singerResults.value, ...newSingers]
        singerCurrentPage.value = nextPage
        singerHasMore.value = response.has_more
    } catch (error) {
        console.error('加载更多歌手失败:', error)
    } finally {
        singerLoadingMore.value = false
    }
}

async function openSinger(singer: SingerInfo) {
    await openSingerByMid(singer.mid, singer.name, singer)
}

async function openSingerByMid(mid: string, name: string, preview?: SingerInfo) {
    singerDetail.value = true
    singerDetailLoading.value = true
    singerDetailError.value = ''
    singerAlbums.value = []
    singerAlbumsTotal.value = 0
    singerAlbumsBegin.value = 0
    singerAlbumsHasMore.value = false
    singerDetailInfo.value = preview ?? {
        id: 0,
        mid,
        name,
        coverUrl: '',
        albumCount: 0,
        songCount: 0,
    }
    hasSearched.value = true
    searchType.value = 'singer'
    albumOpenedFromSinger.value = false

    try {
        const res = await musicApi.fetchSingerAlbums(mid, 0, PAGE_SIZE)
        singerAlbums.value = res.albums
        singerAlbumsTotal.value = res.total
        singerAlbumsBegin.value = res.albums.length
        singerAlbumsHasMore.value = res.has_more
        if (singerDetailInfo.value && res.total > 0) {
            singerDetailInfo.value = {
                ...singerDetailInfo.value,
                albumCount: res.total,
            }
        }
    } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e)
        singerDetailError.value = message || '获取歌手专辑失败'
    } finally {
        singerDetailLoading.value = false
    }
}

async function loadMoreSingerAlbums() {
    const singer = singerDetailInfo.value
    if (!singer || singerDetailLoading.value || singerAlbumsLoadingMore.value || !singerAlbumsHasMore.value) {
        return
    }

    singerAlbumsLoadingMore.value = true
    try {
        const res = await musicApi.fetchSingerAlbums(singer.mid, singerAlbumsBegin.value, PAGE_SIZE)
        const existingIds = new Set(singerAlbums.value.map((a) => a.mid))
        const newAlbums = res.albums.filter((a) => !existingIds.has(a.mid))
        singerAlbums.value = [...singerAlbums.value, ...newAlbums]
        singerAlbumsBegin.value += res.albums.length
        singerAlbumsHasMore.value = res.has_more
        if (res.total > 0) {
            singerAlbumsTotal.value = res.total
        }
    } catch (error) {
        console.error('加载更多歌手专辑失败:', error)
    } finally {
        singerAlbumsLoadingMore.value = false
    }
}

function resetSingerDetail() {
    singerDetail.value = false
    singerDetailInfo.value = null
    singerAlbums.value = []
    singerAlbumsTotal.value = 0
    singerAlbumsBegin.value = 0
    singerAlbumsHasMore.value = false
    singerAlbumsLoadingMore.value = false
    singerDetailError.value = ''
    singerDetailLoading.value = false
    downloadingAlbumMid.value = ''
    albumOpenedFromSinger.value = false
}

function closeSingerDetail() {
    const shouldFillSingerList =
        singerDetail.value &&
        searchType.value === 'singer' &&
        singerResults.value.length === 0 &&
        keyword.value.trim() !== ''

    resetSingerDetail()

    if (shouldFillSingerList) {
        handleSearch()
    }
}

async function openAlbum(album: AlbumInfo) {
    albumOpenedFromSinger.value = singerDetail.value
    await openAlbumByMid(album.mid, album.name, album.artist, album)
}

async function openAlbumByMid(
    mid: string,
    name: string,
    artist: string,
    preview?: AlbumInfo
) {
    albumDetail.value = true
    albumDetailLoading.value = true
    albumDetailError.value = ''
    albumSongs.value = []
    albumSelectedIds.value = []
    albumDetailInfo.value = preview ?? {
        id: 0,
        mid,
        name,
        artist,
        coverUrl: '',
        songCount: 0,
        publishTime: '',
    }
    hasSearched.value = true
    if (!albumOpenedFromSinger.value) {
        searchType.value = 'album'
    }

    try {
        const res = await musicApi.fetchAlbumSongs(mid)
        albumDetailInfo.value = res.album
        albumSongs.value = res.songs
    } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e)
        albumDetailError.value = message || '获取专辑失败'
    } finally {
        albumDetailLoading.value = false
    }
}

function resetAlbumDetail() {
    albumDetail.value = false
    albumDetailInfo.value = null
    albumSongs.value = []
    albumSelectedIds.value = []
    albumDetailError.value = ''
    albumDetailLoading.value = false
}

function closeAlbumDetail() {
    const fromSinger = albumOpenedFromSinger.value
    const shouldFillAlbumList =
        !fromSinger &&
        albumDetail.value &&
        searchType.value === 'album' &&
        albumResults.value.length === 0 &&
        keyword.value.trim() !== ''

    resetAlbumDetail()
    albumOpenedFromSinger.value = false

    if (fromSinger) {
        return
    }

    if (shouldFillAlbumList) {
        handleSearch()
    }
}

function onSingleDownload(song: SongInfo) {
    downloadSingle(enrichSongsWithAlbumMeta([song])[0])
}

function enrichSongsWithAlbumMeta(songs: SongInfo[], album?: AlbumInfo | null): SongInfo[] {
    const info = album ?? albumDetailInfo.value
    if (!info) return songs
    return songs.map((s) => ({
        ...s,
        album: info.name?.trim() || s.album,
        albumArtist: info.artist || singerDetailInfo.value?.name || s.albumArtist,
        albumPublishTime: info.publishTime,
        albumSongCount: info.songCount || songs.length,
    }))
}

function onBatchDownload() {
    const songs = searchResults.value.filter((s) => selectedIds.value.includes(s.mid))
    if (songs.length > 0) {
        batchDownload(songs)
    }
}

function onAlbumBatchDownload(songs: SongInfo[]) {
    if (songs.length > 0) {
        batchDownload(enrichSongsWithAlbumMeta(songs))
    }
}

function onDownloadAlbum(songs: SongInfo[]) {
    if (songs.length === 0) return
    batchDownload(enrichSongsWithAlbumMeta(songs), { useMusicLibrary: true })
}

async function onDownloadSingerAlbum(album: AlbumInfo) {
    if (downloadingAlbumMid.value) return
    downloadingAlbumMid.value = album.mid
    try {
        const res = await musicApi.fetchAlbumSongs(album.mid)
        const albumInfo = {
            ...res.album,
            artist: res.album.artist || album.artist || singerDetailInfo.value?.name || '',
            publishTime: res.album.publishTime || album.publishTime,
            songCount: res.album.songCount || album.songCount || res.songs.length,
        }
        const songs = enrichSongsWithAlbumMeta(res.songs, albumInfo)
        if (songs.length === 0) {
            notification.warning({
                title: '无法下载',
                description: `专辑「${album.name}」没有可下载的歌曲`,
                duration: 3000,
            })
            return
        }
        await batchDownload(songs, { useMusicLibrary: true })
    } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e)
        notification.error({
            title: '下载失败',
            description: message || `获取专辑「${album.name}」失败`,
            duration: 4000,
        })
    } finally {
        downloadingAlbumMid.value = ''
    }
}
</script>

<style scoped>
.search-view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    padding-bottom: 0;
}

.search-header {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
}

.search-header :deep(.search-bar) {
    margin-bottom: 0;
}

.search-type-group {
    width: 100%;
    display: flex;
}

.search-type-group :deep(.n-radio-button) {
    flex: 1;
    justify-content: center;
}

.loading-wrapper {
    display: flex;
    justify-content: center;
    padding: 40px 0;
}
</style>
