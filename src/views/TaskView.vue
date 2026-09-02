<template>
    <div class="task-view">
        <div class="task-header">
            <TaskTabs v-model:activeTab="activeTab" :counts="tabCounts" />
        </div>

        <div class="task-table-wrap">
            <TaskTable :tasks="filteredTasks" :selectedRowKeys="selectedRowKeys"
                @update:selectedRowKeys="selectedRowKeys = $event" @action="handleAction" />
        </div>

        <TaskBatchActions :selectedCount="selectedRowKeys.length" :canClearFinished="canClearFinished"
            @clear="handleBatchClear" @clear-completed="openClearModal" />

        <n-modal v-model:show="showClearModal" preset="dialog" title="清除已完成任务" positive-text="确定"
            negative-text="取消" @positive-click="confirmClearFinished">
            <n-space vertical :size="12">
                <span>将从任务列表中移除已完成的任务。</span>
                <n-checkbox v-model:checked="clearDeleteFile">同时删除文件</n-checkbox>
                <n-checkbox v-model:checked="clearFailed">同时清除失败的下载项</n-checkbox>
            </n-space>
        </n-modal>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NModal, NCheckbox, NSpace } from 'naive-ui'
import { useTaskStore } from '../stores/taskStore'
import { useDownloadActions } from '../composables/useDownloadActions'
import TaskTabs from '../components/task/TaskTabs.vue'
import TaskTable from '../components/task/TaskTable.vue'
import TaskBatchActions from '../components/task/TaskBatchActions.vue'

const taskStore = useTaskStore()
const { retryTask } = useDownloadActions()

const activeTab = ref('all')
const selectedRowKeys = ref<string[]>([])

const showClearModal = ref(false)
const clearDeleteFile = ref(false)
const clearFailed = ref(false)

const tabCounts = computed(() => {
    const counts = {
        total: 0,
        waiting: 0,
        downloading: 0,
        paused: 0,
        completed: 0,
        error: 0,
    }
    for (const task of taskStore.tasks) {
        counts.total++
        if (task.status === 'waiting') counts.waiting++
        else if (task.status === 'downloading') counts.downloading++
        else if (task.status === 'paused') counts.paused++
        else if (task.status === 'completed') counts.completed++
        else if (task.status === 'error') counts.error++
    }
    return counts
})

const canClearFinished = computed(
    () => tabCounts.value.completed > 0 || tabCounts.value.error > 0
)

const filteredTasks = computed(() => {
    const all = taskStore.tasks.filter((t) => {
        // 显式读取进度相关字段，建立响应式依赖
        void t.downloaded;
        void t.fileSize;
        return activeTab.value === 'all' || t.status === activeTab.value;
    });
    return all;
})

function openClearModal() {
    clearDeleteFile.value = false
    clearFailed.value = false
    showClearModal.value = true
}

async function confirmClearFinished() {
    const ids = taskStore.tasks
        .filter((t) => {
            if (t.status === 'completed') return true
            if (clearFailed.value && t.status === 'error') return true
            return false
        })
        .map((t) => t.id)

    const deleteFile = clearDeleteFile.value
    for (const taskId of ids) {
        await taskStore.removeTask(taskId, deleteFile)
    }
    selectedRowKeys.value = selectedRowKeys.value.filter((id) => !ids.includes(id))
}

async function handleAction(action: string, taskId: string, extra?: any) {
    switch (action) {
        case 'cancel':
            taskStore.cancelTask(taskId, extra?.deleteFile === true)
            break
        case 'pause':
            taskStore.pauseTask(taskId)
            break
        case 'resume':
            taskStore.resumeTask(taskId)
            break
        case 'retry':
            await retryTask(taskId)
            break
        case 'remove':
            taskStore.removeTask(taskId, extra?.deleteFile === true)
            break
        case 'open-location': {
            const task = taskStore.tasks.find((t) => t.id === taskId)
            if (task?.filePath) {
                try {
                    await invoke('open_file_location', { path: task.filePath })
                } catch (e) {
                    console.error('打开文件位置失败:', e)
                }
            }
            break
        }
    }
    // 清除相关选中状态
    selectedRowKeys.value = selectedRowKeys.value.filter((id) => id !== taskId)
}

async function handleBatchClear(deleteFile: boolean) {
    const ids = selectedRowKeys.value.slice()
    for (const taskId of ids) {
        const task = taskStore.tasks.find((t) => t.id === taskId)
        if (!task) continue
        if (
            task.status === 'waiting' ||
            task.status === 'downloading' ||
            task.status === 'paused'
        ) {
            taskStore.cancelTask(taskId, deleteFile)
        } else {
            // removeTask 现在接受 deleteFile 参数
            await taskStore.removeTask(taskId, deleteFile)
        }
    }
    selectedRowKeys.value = []
}
</script>

<style scoped>
.task-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
}

.task-header {
    flex-shrink: 0;
}

.task-table-wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
}
</style>
