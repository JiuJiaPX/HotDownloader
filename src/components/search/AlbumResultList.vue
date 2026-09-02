<template>
    <div class="album-result-list">
        <template v-if="albums.length > 0">
            <div class="album-items">
                <AlbumItem v-for="album in albums" :key="album.mid" :album="album" @open="$emit('open', $event)" />
            </div>

            <div v-if="hasMore" class="load-more-wrapper">
                <n-button :loading="loadingMore" :disabled="loadingMore" @click="$emit('load-more')">
                    {{ loadingMore ? '加载中...' : '加载更多' }}
                </n-button>
            </div>
        </template>

        <div v-else class="empty-result">
            <n-empty description="暂无专辑搜索结果" />
            <div class="retry-wrapper">
                <n-button type="primary" @click="$emit('retry')">重试</n-button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { NEmpty, NButton } from 'naive-ui'
import type { AlbumInfo } from '../../types'
import AlbumItem from './AlbumItem.vue'

withDefaults(defineProps<{
    albums: AlbumInfo[]
    hasMore?: boolean
    loadingMore?: boolean
}>(), {
    hasMore: false,
    loadingMore: false,
})

defineEmits<{
    (e: 'open', album: AlbumInfo): void
    (e: 'retry'): void
    (e: 'load-more'): void
}>()
</script>

<style scoped>
.album-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.load-more-wrapper {
    display: flex;
    justify-content: center;
    margin-top: 16px;
}

.empty-result {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px 0;
}

.retry-wrapper {
    margin-top: 16px;
    text-align: center;
}
</style>
