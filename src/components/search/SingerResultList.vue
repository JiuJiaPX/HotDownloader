<template>
    <div class="singer-result-list">
        <template v-if="singers.length > 0">
            <div class="singer-items">
                <SingerItem v-for="singer in singers" :key="singer.mid" :singer="singer"
                    @open="$emit('open', $event)" />
            </div>

            <div v-if="hasMore" class="load-more-wrapper">
                <n-button :loading="loadingMore" :disabled="loadingMore" @click="$emit('load-more')">
                    {{ loadingMore ? '加载中...' : '加载更多' }}
                </n-button>
            </div>
        </template>

        <div v-else class="empty-result">
            <n-empty description="暂无歌手搜索结果" />
            <div class="retry-wrapper">
                <n-button type="primary" @click="$emit('retry')">重试</n-button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { NEmpty, NButton } from 'naive-ui'
import type { SingerInfo } from '../../types'
import SingerItem from './SingerItem.vue'

withDefaults(defineProps<{
    singers: SingerInfo[]
    hasMore?: boolean
    loadingMore?: boolean
}>(), {
    hasMore: false,
    loadingMore: false,
})

defineEmits<{
    (e: 'open', singer: SingerInfo): void
    (e: 'retry'): void
    (e: 'load-more'): void
}>()
</script>

<style scoped>
.singer-items {
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
