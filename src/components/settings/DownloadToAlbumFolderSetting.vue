<template>
    <template v-if="isNarrow">
        <div class="setting-row">
            <span class="setting-label">下载到专辑文件夹</span>
            <n-switch :value="settingsStore.settings.downloadToAlbumFolder"
                @update:value="(val) => (settingsStore.settings.downloadToAlbumFolder = val)" />
        </div>
        <div class="setting-hint">开启后，歌曲会保存到「下载目录 / 专辑名」下；专辑名为空时使用「未知专辑」</div>
    </template>
    <template v-else>
        <n-form-item label="下载到专辑文件夹">
            <n-switch :value="settingsStore.settings.downloadToAlbumFolder"
                @update:value="(val) => (settingsStore.settings.downloadToAlbumFolder = val)" />
            <template #feedback>
                <div class="setting-hint">
                    开启后，歌曲会保存到「下载目录 / 专辑名」下；专辑名为空时使用「未知专辑」
                </div>
            </template>
        </n-form-item>
    </template>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NFormItem, NSwitch } from 'naive-ui'
import { useSettingsStore } from '../../stores/settingsStore'

const settingsStore = useSettingsStore()

const isNarrow = ref(
    typeof window !== 'undefined' &&
    window.matchMedia('(max-width: 767px)').matches
)
let mediaQuery: MediaQueryList | null = null

function updateNarrow(e: MediaQueryListEvent | MediaQueryList) {
    isNarrow.value = e.matches
}

onMounted(() => {
    mediaQuery = window.matchMedia('(max-width: 767px)')
    updateNarrow(mediaQuery)
    mediaQuery.addEventListener('change', updateNarrow)
})

onUnmounted(() => {
    if (mediaQuery) {
        mediaQuery.removeEventListener('change', updateNarrow)
    }
})
</script>

<style scoped>
.setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}

.setting-label {
    font-size: 14px;
    color: var(--n-text-color);
}

.setting-hint {
    font-size: 12px;
    color: var(--n-text-color-3);
    line-height: 1.6;
}

.setting-row + .setting-hint {
    margin-bottom: 16px;
}
</style>
