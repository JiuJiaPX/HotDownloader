<template>
    <div class="singer-item" @click="$emit('open', singer)">
        <img v-if="singer.coverUrl" :src="singer.coverUrl" class="cover" alt="歌手头像" />
        <div v-else class="cover cover-placeholder" />
        <div class="info">
            <div class="title">{{ singer.name }}</div>
            <div class="meta">
                <span v-if="singer.albumCount">{{ singer.albumCount }} 张专辑</span>
                <span v-if="singer.albumCount && singer.songCount"> · </span>
                <span v-if="singer.songCount">{{ singer.songCount }} 首歌曲</span>
            </div>
        </div>
        <n-button size="small" @click.stop="$emit('open', singer)">
            查看
        </n-button>
    </div>
</template>

<script setup lang="ts">
import { NButton } from 'naive-ui'
import type { SingerInfo } from '../../types'

defineProps<{
    singer: SingerInfo
}>()

defineEmits<{
    (e: 'open', singer: SingerInfo): void
}>()
</script>

<style scoped>
.singer-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border: 1px solid var(--n-border-color, #eee);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s;
}

.singer-item:hover {
    background: var(--n-color-hover, rgba(0, 0, 0, 0.04));
}

.info {
    flex: 1;
    overflow: hidden;
}

.cover {
    width: 56px;
    height: 56px;
    border-radius: 50%;
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

.meta {
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-top: 4px;
}
</style>
