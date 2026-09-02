<template>
    <div class="album-item" @click="$emit('open', album)">
        <img v-if="album.coverUrl" :src="album.coverUrl" class="cover" alt="专辑封面" />
        <div v-else class="cover cover-placeholder" />
        <div class="info">
            <div class="title">{{ album.name }}</div>
            <div class="subtitle">{{ album.artist }}</div>
            <div class="meta">
                <span v-if="album.songCount">{{ album.songCount }} 首</span>
                <span v-if="album.songCount && album.publishTime"> · </span>
                <span v-if="album.publishTime">{{ album.publishTime }}</span>
            </div>
        </div>
        <n-button size="small" @click.stop="$emit('open', album)">
            查看
        </n-button>
    </div>
</template>

<script setup lang="ts">
import { NButton } from 'naive-ui'
import type { AlbumInfo } from '../../types'

defineProps<{
    album: AlbumInfo
}>()

defineEmits<{
    (e: 'open', album: AlbumInfo): void
}>()
</script>

<style scoped>
.album-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border: 1px solid var(--n-border-color, #eee);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s;
}

.album-item:hover {
    background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
}

.info {
    flex: 1;
    overflow: hidden;
}

.cover {
    width: 56px;
    height: 56px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
}

.cover-placeholder {
    background: var(--border-color, #e0e0e0);
}

.title {
    font-size: 15px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-text);
}

.subtitle {
    font-size: 13px;
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.meta {
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-top: 4px;
}
</style>
