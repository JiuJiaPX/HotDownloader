<template>
    <div class="batch-actions">
        <n-button size="small" :disabled="!canClearFinished" @click="$emit('clear-completed')">
            清除已完成
        </n-button>
        <div v-if="selectedCount > 0" class="batch-actions-selected">
            <span>已选择 {{ selectedCount }} 个任务</span>
            <n-popconfirm @positive-click="handleConfirm">
                <template #trigger>
                    <n-button type="error" size="small">清除所选</n-button>
                </template>
                <n-space vertical :size="8">
                    <span>确认清除所选任务吗？</span>
                    <n-checkbox v-model:checked="deleteFile">
                        同时删除已下载或未完成的文件
                    </n-checkbox>
                </n-space>
            </n-popconfirm>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NPopconfirm, NCheckbox, NSpace } from 'naive-ui'

defineProps<{
    selectedCount: number
    canClearFinished: boolean
}>()

const emit = defineEmits<{
    (e: 'clear', deleteFile: boolean): void
    (e: 'clear-completed'): void
}>()

const deleteFile = ref(false)

function handleConfirm() {
    emit('clear', deleteFile.value)
    deleteFile.value = false
}
</script>

<style scoped>
.batch-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-bottom, var(--n-color, #fff));
    border-top: 1px solid var(--border-color, var(--n-border-color, #e0e0e0));
    flex-shrink: 0;
}

.batch-actions-selected {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-left: auto;
}
</style>
