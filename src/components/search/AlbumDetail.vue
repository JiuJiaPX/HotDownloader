<template>
    <div class="album-detail">
        <n-button text class="back-btn" @click="$emit('back')">返回专辑列表</n-button>

        <div v-if="loading" class="loading-wrapper">
            <n-spin size="medium" />
        </div>

        <div v-else-if="errorMsg" class="error-wrapper">
            <n-alert type="error" :title="errorMsg" />
        </div>

        <template v-else-if="album">
            <div class="album-info">
                <img v-if="album.coverUrl" :src="album.coverUrl" class="album-cover" alt="专辑封面" />
                <div v-else class="album-cover cover-placeholder" />
                <div class="album-details">
                    <div class="album-name">{{ album.name }}</div>
                    <div class="album-artist">{{ album.artist }}</div>
                    <div class="album-meta">
                        <span v-if="album.songCount">{{ album.songCount }} 首</span>
                        <span v-if="album.songCount && album.publishTime"> · </span>
                        <span v-if="album.publishTime">{{ album.publishTime }}</span>
                    </div>
                </div>
                <n-button type="primary" class="album-download-btn" :disabled="songs.length === 0"
                    title="保存到本机音乐库 / HotDownloader / 专辑名" @click="onDownloadAlbum">
                    下载整张专辑
                </n-button>
            </div>

            <div class="list-header">
                <n-checkbox :checked="isAllSelected" :indeterminate="isIndeterminate" @update:checked="toggleAll">
                    全选
                </n-checkbox>
                <span class="count-text">已选 {{ selectedIds.length }} / {{ songs.length }} 首</span>
            </div>

            <div class="song-items">
                <SongItem v-for="song in songs" :key="song.mid" :song="song" :selected="selectedIds.includes(song.mid)"
                    @toggle-select="(val) => toggleSelect(song.mid, val)" @download="(song) => $emit('download', song)" />
            </div>

            <BatchDownloadBar v-if="selectedIds.length > 0" :selectedCount="selectedIds.length"
                @batch-download="onBatchDownload" />
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NSpin, NAlert, NCheckbox } from 'naive-ui'
import type { AlbumInfo, SongInfo } from '../../types'
import SongItem from './SongItem.vue'
import BatchDownloadBar from './BatchDownloadBar.vue'

const props = defineProps<{
    album: AlbumInfo | null
    songs: SongInfo[]
    loading: boolean
    errorMsg: string
    selectedIds: string[]
}>()

const emit = defineEmits<{
    (e: 'back'): void
    (e: 'download', song: SongInfo): void
    (e: 'batch-download', songs: SongInfo[]): void
    (e: 'download-album', songs: SongInfo[]): void
    (e: 'update:selectedIds', ids: string[]): void
}>()

const isAllSelected = computed(
    () => props.songs.length > 0 && props.selectedIds.length === props.songs.length
)

const isIndeterminate = computed(
    () => props.selectedIds.length > 0 && props.selectedIds.length < props.songs.length
)

function toggleAll(checked: boolean) {
    emit('update:selectedIds', checked ? props.songs.map((s) => s.mid) : [])
}

function toggleSelect(songMid: string, selected: boolean) {
    if (selected) {
        emit('update:selectedIds', [...props.selectedIds, songMid])
    } else {
        emit('update:selectedIds', props.selectedIds.filter((id) => id !== songMid))
    }
}

function onBatchDownload() {
    const selected = props.songs.filter((s) => props.selectedIds.includes(s.mid))
    if (selected.length > 0) {
        emit('batch-download', selected)
    }
}

function onDownloadAlbum() {
    if (props.songs.length > 0) {
        emit('download-album', props.songs)
    }
}
</script>

<style scoped>
.album-detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.back-btn {
    align-self: flex-start;
}

.loading-wrapper,
.error-wrapper {
    display: flex;
    justify-content: center;
    padding: 40px 0;
}

.album-info {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 12px;
    background-color: var(--bg-sidebar);
    border-radius: 8px;
    flex-wrap: wrap;
}

.album-cover {
    width: 80px;
    height: 80px;
    border-radius: 8px;
    object-fit: cover;
    flex-shrink: 0;
}

.cover-placeholder {
    background: var(--border-color, #e0e0e0);
}

.album-details {
    flex: 1;
    min-width: 0;
}

.album-download-btn {
    flex-shrink: 0;
    margin-left: auto;
}

.album-name {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 8px;
    color: var(--color-text);
}

.album-artist {
    color: var(--color-text-secondary);
    font-size: 14px;
}

.album-meta {
    color: var(--color-text-secondary);
    font-size: 13px;
    margin-top: 4px;
}

.list-header {
    display: flex;
    align-items: center;
    gap: 12px;
}

.count-text {
    font-size: 13px;
    color: var(--color-text-secondary);
}

.song-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
}
</style>
