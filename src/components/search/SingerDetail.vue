<template>
    <div class="singer-detail">
        <n-button text class="back-btn" @click="$emit('back')">返回歌手列表</n-button>

        <div v-if="loading && albums.length === 0" class="loading-wrapper">
            <n-spin size="medium" />
        </div>

        <div v-else-if="errorMsg && albums.length === 0" class="error-wrapper">
            <n-alert type="error" :title="errorMsg" />
        </div>

        <template v-else-if="singer">
            <div class="singer-info">
                <img v-if="singer.coverUrl" :src="singer.coverUrl" class="singer-cover" alt="歌手头像" />
                <div v-else class="singer-cover cover-placeholder" />
                <div class="singer-details">
                    <div class="singer-name">{{ singer.name }}</div>
                    <div class="singer-meta">
                        <span v-if="total > 0">{{ total }} 张专辑</span>
                        <span v-else-if="singer.albumCount">{{ singer.albumCount }} 张专辑</span>
                        <span v-if="(total || singer.albumCount) && singer.songCount"> · </span>
                        <span v-if="singer.songCount">{{ singer.songCount }} 首歌曲</span>
                    </div>
                </div>
            </div>

            <div v-if="albums.length === 0" class="empty-wrapper">
                <n-empty description="暂无专辑" />
            </div>

            <div v-else class="album-cards">
                <div v-for="album in albums" :key="album.mid" class="album-card">
                    <div class="album-row" @click="$emit('open-album', album)">
                        <img v-if="album.coverUrl" :src="album.coverUrl" class="album-cover" alt="专辑封面" />
                        <div v-else class="album-cover cover-placeholder" />
                        <div class="album-text">
                            <div class="album-name">{{ album.name }}</div>
                            <div class="album-sub">
                                <span v-if="album.songCount">{{ album.songCount }} 首</span>
                                <span v-if="album.songCount && album.publishTime"> · </span>
                                <span v-if="album.publishTime">{{ album.publishTime }}</span>
                            </div>
                        </div>
                        <n-button size="small" @click.stop="$emit('open-album', album)">查看</n-button>
                    </div>
                    <n-button
                        type="primary"
                        block
                        class="download-album-btn"
                        :loading="downloadingMid === album.mid"
                        :disabled="!!downloadingMid"
                        @click="$emit('download-album', album)"
                    >
                        下载整张专辑
                    </n-button>
                </div>
            </div>

            <div v-if="hasMore" class="load-more-wrapper">
                <n-button :loading="loadingMore" :disabled="loadingMore" @click="$emit('load-more')">
                    {{ loadingMore ? '加载中...' : '加载更多专辑' }}
                </n-button>
            </div>
        </template>
    </div>
</template>

<script setup lang="ts">
import { NButton, NSpin, NAlert, NEmpty } from 'naive-ui'
import type { AlbumInfo, SingerInfo } from '../../types'

withDefaults(defineProps<{
    singer: SingerInfo | null
    albums: AlbumInfo[]
    total?: number
    loading?: boolean
    loadingMore?: boolean
    hasMore?: boolean
    errorMsg?: string
    downloadingMid?: string
}>(), {
    total: 0,
    loading: false,
    loadingMore: false,
    hasMore: false,
    errorMsg: '',
    downloadingMid: '',
})

defineEmits<{
    (e: 'back'): void
    (e: 'open-album', album: AlbumInfo): void
    (e: 'download-album', album: AlbumInfo): void
    (e: 'load-more'): void
}>()
</script>

<style scoped>
.singer-detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.back-btn {
    align-self: flex-start;
}

.loading-wrapper,
.error-wrapper,
.empty-wrapper {
    display: flex;
    justify-content: center;
    padding: 40px 0;
}

.singer-info {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 12px;
    background-color: var(--bg-sidebar);
    border-radius: 8px;
}

.singer-cover {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
}

.cover-placeholder {
    background: var(--border-color, #e0e0e0);
}

.singer-details {
    flex: 1;
    min-width: 0;
}

.singer-name {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 8px;
    color: var(--color-text);
}

.singer-meta {
    color: var(--color-text-secondary);
    font-size: 13px;
}

.album-cards {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.album-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--n-border-color, #eee);
    border-radius: 8px;
}

.album-row {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
}

.album-cover {
    width: 56px;
    height: 56px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
}

.album-text {
    flex: 1;
    min-width: 0;
}

.album-name {
    font-size: 15px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-text);
}

.album-sub {
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-top: 4px;
}

.download-album-btn {
    margin-top: 0;
}

.load-more-wrapper {
    display: flex;
    justify-content: center;
}
</style>
