<template>
    <div class="search-view">
        <div v-if="!albumDetail" class="search-header">
            <n-radio-group :value="searchType" size="medium" class="search-type-group"
                @update:value="onSearchTypeChange">
                <n-radio-button value="song">搜索歌曲</n-radio-button>
                <n-radio-button value="album">搜索专辑</n-radio-button>
            </n-radio-group>
            <SearchBar v-model:keyword="keyword" :placeholder="searchPlaceholder" @search="handleSearch" />
        </div>

        <!-- 输入非空且未搜索：显示搜索建议 -->
        <SearchSuggestions v-if="showSuggestions" :data="suggestions" @select="onSuggestionSelect"
            @select-album="onSuggestionAlbumSelect" />

        <!-- 输入为空且未搜索：显示历史与热搜 -->
        <div v-if="!keyword && !hasSearched && !albumDetail">
            <SearchHistory :history="historyStore.history" @select="onHistorySelect" @remove="onHistoryRemove"
                @clear="historyStore.clearHistory" />
            <HotKeywords :keywords="hotKeywords" :loading="hotLoading" @select="onHotClick" />
        </div>

        <!-- 加载中 -->
        <div v-if="loading && !albumDetail" class="loading-wrapper">
            <n-spin size="medium" />
        </div>

        <!-- 歌曲搜索结果 -->
        <SearchResultList v-if="hasSearched && !loading && searchType === 'song' && !albumDetail"
            :songs="searchResults" v-model:selectedIds="selectedIds" :has-more="hasMore" :loading-more="loadingMore"
            @download="onSingleDownload" @retry="handleSearch" @load-more="loadMore" />

        <!-- 专辑搜索结果 -->
        <AlbumResultList v-if="hasSearched && !loading && searchType === 'album' && !albumDetail"
            :albums="albumResults" :has-more="albumHasMore" :loading-more="albumLoadingMore" @open="openAlbum"
            @retry="handleSearch" @load-more="loadMoreAlbums" />

        <!-- 专辑详情 -->
        <AlbumDetail v-if="albumDetail" :album="albumDetailInfo" :songs="albumSongs" :loading="albumDetailLoading"
            :error-msg="albumDetailError" v-model:selectedIds="albumSelectedIds" @back="closeAlbumDetail"
            @download="onSingleDownload" @batch-download="onAlbumBatchDownload"
            @download-album="onDownloadAlbum" />

        <BatchDownloadBar v-if="selectedIds.length > 0 && !albumDetail" :selectedCount="selectedIds.length"
            @batch-download="onBatchDownload" />
    </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue'
import { NSpin, NRadioGroup, NRadioButton } from 'naive-ui'
import SearchBar from '../components/search/SearchBar.vue'
import SearchHistory from '../components/search/SearchHistory.vue'
import HotKeywords from '../components/search/HotKeywords.vue'
import SearchSuggestions from '../components/search/SearchSuggestions.vue'
import SearchResultList from '../components/search/SearchResultList.vue'
import AlbumResultList from '../components/search/AlbumResultList.vue'
import AlbumDetail from '../components/search/AlbumDetail.vue'
import BatchDownloadBar from '../components/search/BatchDownloadBar.vue'
import { useHistoryStore } from '../stores/historyStore'
import { useDownloadActions } from '../composables/useDownloadActions'
import * as musicApi from '../api/musicApi'
import type { SongInfo, SearchSuggestionData, SearchSuggestionItem, AlbumInfo } from '../types'

const keyword = ref('')
const searchType = ref<'song' | 'album'>('song')
const searchResults = ref<SongInfo[]>([])
const albumResults = ref<AlbumInfo[]>([])
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

const albumDetail = ref(false)
const albumDetailInfo = ref<AlbumInfo | null>(null)
const albumSongs = ref<SongInfo[]>([])
const albumDetailLoading = ref(false)
const albumDetailError = ref('')
const albumSelectedIds = ref<string[]>([])

const hotKeywords = ref<string[]>([])
const hotLoading = ref(false)

const historyStore = useHistoryStore()
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
    return keyword.value.trim() !== '' && !hasSearched.value && !albumDetail.value
})

const searchPlaceholder = computed(() => {
    return searchType.value === 'album' ? '搜索专辑' : '搜索歌曲、歌手、专辑'
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

watch(keyword, (newVal) => {
    if (!newVal) {
        hasSearched.value = false
        searchResults.value = []
        albumResults.value = []
        selectedIds.value = []
        suggestions.value = { song: [], singer: [], album: [], mv: [] }
        currentPage.value = 1
        hasMore.value = false
        loadingMore.value = false
        albumCurrentPage.value = 1
        albumHasMore.value = false
        albumLoadingMore.value = false
        lastSongKeyword.value = ''
        lastAlbumKeyword.value = ''
        resetAlbumDetail()
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
    if (type !== 'song' && type !== 'album') return
    if (searchType.value === type) return
    searchType.value = type
    selectedIds.value = []
    const term = keyword.value.trim()
    if (!hasSearched.value || !term) return
    if (type === 'song' && lastSongKeyword.value === term) return
    if (type === 'album' && lastAlbumKeyword.value === term) return
    handleSearch()
}

async function handleSearch() {
    const term = keyword.value.trim()
    if (!term) return

    resetAlbumDetail()
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

async function openAlbum(album: AlbumInfo) {
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
    searchType.value = 'album'

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
    const shouldFillAlbumList =
        albumDetail.value &&
        searchType.value === 'album' &&
        albumResults.value.length === 0 &&
        keyword.value.trim() !== ''

    resetAlbumDetail()

    if (shouldFillAlbumList) {
        handleSearch()
    }
}

function onSingleDownload(song: SongInfo) {
    downloadSingle(song)
}

function onBatchDownload() {
    const songs = searchResults.value.filter((s) => selectedIds.value.includes(s.mid))
    if (songs.length > 0) {
        batchDownload(songs)
    }
}

function onAlbumBatchDownload(songs: SongInfo[]) {
    if (songs.length > 0) {
        batchDownload(songs)
    }
}

function onDownloadAlbum(songs: SongInfo[]) {
    if (songs.length === 0) return
    const albumName = albumDetailInfo.value?.name?.trim() || ''
    const withAlbum = songs.map((s) => ({
        ...s,
        album: albumName || s.album,
    }))
    batchDownload(withAlbum, { useMusicLibrary: true })
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
