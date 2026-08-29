<template>
    <div class="settings-view" :class="{ 'is-narrow': isNarrow }">
        <!-- 移动端：分组纵向布局 -->
        <template v-if="isNarrow">
            <!-- 账号设置：独立分类，位于基本设置上方，增加底部间距避免与下方黏连 -->
            <div class="settings-section account-section">
                <h2 class="section-title">账号设置</h2>
                <n-form label-placement="top">
                    <LoginSetting />
                </n-form>
            </div>

            <div class="settings-section">
                <h2 class="section-title">基本设置</h2>
                <n-form label-placement="top">
                    <QualitySetting />
                    <DowngradeSetting />
                    <ClearHistoryButton />
                </n-form>
            </div>

            <div class="settings-section">
                <h2 class="section-title">下载设置</h2>
                <n-form label-placement="top">
                    <DirectorySetting />
                    <NamingTemplate />
                    <WriteMetadataSetting />
                    <DownloadLrcSetting />
                    <ConcurrencySetting />
                    <JumpToTaskSetting />
                    <DuplicateStrategySetting />
                </n-form>
            </div>
        </template>

        <!-- 桌面端：原有左右分栏表单 -->
        <template v-else>
            <!-- 账号设置：独立分类，位于基本设置上方，增加底部间距避免与下方黏连 -->
            <div class="settings-section account-section">
                <h2 class="section-title">账号设置</h2>
                <LoginSetting />
            </div>

            <n-form label-placement="left" label-width="180">
                <QualitySetting />
                <DowngradeSetting />
                <DirectorySetting />
                <NamingTemplate />
                <WriteMetadataSetting />
                <DownloadLrcSetting />
                <ConcurrencySetting />
                <JumpToTaskSetting />
                <DuplicateStrategySetting />
                <ClearHistoryButton />
            </n-form>
        </template>

        <!-- 检查更新入口（移动端与桌面端通用） -->
        <div class="check-update-entry">
            <n-button :loading="checkingUpdate" @click="handleCheckUpdate">
                检查更新
            </n-button>
            <span v-if="updateInfo && !checkingUpdate" class="update-status-text">
                {{ isNewVersion ? '发现新版本' : '已是最新版本' }}
            </span>
        </div>

        <!-- 关于入口（始终位于页面底部） -->
        <div class="about-entry">
            <n-button text @click="goAbout">关于 HotDownloader</n-button>
        </div>

        <!-- 更新信息弹窗：桌面端最大宽度 600px，移动端左右留白 16px -->
        <n-modal v-model:show="showUpdateModal" preset="card" class="update-modal"
            :title="isNewVersion ? '发现新版本' : '检查更新'" style="max-width: 600px; width: calc(100% - 32px);">
            <div v-if="updateInfo" class="update-content">
                <p class="version-line">
                    当前版本：{{ updateInfo.current_version }}
                    <template v-if="isNewVersion">
                        ｜ 最新版本：{{ updateInfo.tag_name }}
                    </template>
                </p>
                <p v-if="updateInfo.published_at" class="publish-date">
                    发布时间：{{ updateInfo.published_at }}
                </p>
                <div class="update-body">
                    <n-text class="body-label">更新内容：</n-text>
                    <!-- 使用 v-html 渲染 Markdown 解析后的 HTML，提升可读性 -->
                    <!-- 调用 renderMarkdown 函数生成安全 HTML；若无内容则显示默认文本 -->
                    <div class="body-text markdown-body" v-html="renderMarkdown(updateInfo.body) || '<p>（无更新说明）</p>'">
                    </div>
                </div>
                <!-- 下载安装包直链区域（当存在匹配当前平台的 assets 时显示） -->
                <!-- 检查更新功能优化，只显示当前平台可用的安装包，避免用户下载错误文件 -->
                <div v-if="filteredAssets.length > 0" class="assets-section">
                    <n-text class="body-label">下载安装包：</n-text>
                    <div class="asset-list">
                        <!-- 使用 filteredAssets 计算属性，其根据 currentPlatform 过滤原始 assets -->
                        <a v-for="asset in filteredAssets" :key="asset.name" class="asset-link"
                            :href="asset.browser_download_url" target="_blank" rel="noopener noreferrer">
                            {{ asset.name }}（{{ formatFileSize(asset.size) }}）
                        </a>
                    </div>
                </div>
                <div class="modal-actions">
                    <n-button type="primary" @click="showUpdateModal = false">关闭</n-button>
                    <n-button v-if="updateInfo.html_url" tag="a" :href="updateInfo.html_url" target="_blank">
                        前往发布页
                    </n-button>
                </div>
            </div>
        </n-modal>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { NForm, NButton, NModal, NText } from 'naive-ui'
// Markdown 渲染依赖
import { marked } from 'marked'
import DOMPurify from 'dompurify'
// 语义化版本比较库 semver
import semver from 'semver'
// Tauri OS 插件，用于获取当前平台信息
import { platform } from '@tauri-apps/plugin-os'
import QualitySetting from '../components/settings/QualitySetting.vue'
import DowngradeSetting from '../components/settings/DowngradeSetting.vue'
import DirectorySetting from '../components/settings/DirectorySetting.vue'
import NamingTemplate from '../components/settings/NamingTemplate.vue'
import ConcurrencySetting from '../components/settings/ConcurrencySetting.vue'
import JumpToTaskSetting from '../components/settings/JumpToTaskSetting.vue'
import ClearHistoryButton from '../components/settings/ClearHistoryButton.vue'
import WriteMetadataSetting from '../components/settings/WriteMetadataSetting.vue'
import DownloadLrcSetting from '../components/settings/DownloadLrcSetting.vue'
import LoginSetting from '../components/settings/LoginSetting.vue'
import DuplicateStrategySetting from '../components/settings/DuplicateStrategySetting.vue'
import * as musicApi from '../api/musicApi'
import type { UpdateInfo } from '../types'

const router = useRouter()

const isNarrow = ref(
    typeof window !== 'undefined' &&
    window.matchMedia('(max-width: 767px)').matches
)

let mediaQuery: MediaQueryList | null = null

function updateNarrow(e: MediaQueryListEvent | MediaQueryList) {
    isNarrow.value = e.matches
}

onMounted(async () => {
    mediaQuery = window.matchMedia('(max-width: 767px)')
    updateNarrow(mediaQuery)
    mediaQuery.addEventListener('change', updateNarrow)

    // 使用 async/await 获取当前平台信息，避免类型不匹配和代码繁琐
    try {
        currentPlatform.value = await platform()
    } catch (err) {
        console.warn('获取平台信息失败，将显示所有安装包', err)
        currentPlatform.value = ''
    }
})

onUnmounted(() => {
    if (mediaQuery) {
        mediaQuery.removeEventListener('change', updateNarrow)
    }
})

function goAbout() {
    router.push('/settings/about')
}

// 检查更新相关
const checkingUpdate = ref(false)
const updateInfo = ref<UpdateInfo | null>(null)
const showUpdateModal = ref(false)

// 当前平台标识，用于过滤下载资产
const currentPlatform = ref<string>('')

// 根据当前平台过滤下载资产列表，实现平台相关的安装包显示，提升用户体验
const filteredAssets = computed(() => {
    if (!updateInfo.value || !updateInfo.value.assets) return []
    const platformStr = currentPlatform.value
    if (!platformStr) return updateInfo.value.assets // 平台尚未获取，显示全部
    const assets = updateInfo.value.assets
    if (platformStr === 'android') {
        // Android 平台仅展示 release 版本的 APK
        return assets.filter(a => a.name.endsWith('.apk') && a.name.toLowerCase().includes('release'))
    } else if (platformStr === 'windows') {
        return assets.filter(a => a.name.endsWith('.exe') || a.name.endsWith('.msi'))
    } else if (platformStr === 'macos' || platformStr === 'darwin') {
        return assets.filter(a => a.name.endsWith('.dmg'))
    } else if (platformStr === 'linux') {
        return assets.filter(a => a.name.endsWith('.deb') || a.name.endsWith('.rpm') || a.name.endsWith('.AppImage'))
    }
    // 其他平台显示全部
    return assets
})

// 支持更新内容 Markdown 渲染，提升可读性
function renderMarkdown(markdown: string): string {
    if (!markdown) return ''
    // 配置 marked 渲染选项：启用 GFM（GitHub Flavored Markdown）和换行转换
    marked.setOptions({ gfm: true, breaks: true })
    const rawHtml = marked.parse(markdown) as string
    // 使用 DOMPurify 过滤，允许常见安全标签
    return DOMPurify.sanitize(rawHtml, { USE_PROFILES: { html: true } })
}

// 将字节大小格式化为人类可读字符串
function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    const value = bytes / Math.pow(1024, i)
    return `${value.toFixed(2)} ${units[i]}`
}

// 检查更新功能优化，使用成熟库替代手写比较，正确处理预发布版本等复杂情况
// 去除 tag_name 可能的前缀 v，然后使用 semver.gt() 判断 latest 是否大于 current
const isNewVersion = computed(() => {
    if (!updateInfo.value) return false
    const current = updateInfo.value.current_version
    const latest = updateInfo.value.tag_name.replace(/^v/, '')
    // 使用 semver.gt 比较两个版本号，返回是否 latest > current
    // 注意：semver 库会处理预发布版本的优先级规则
    return semver.valid(current) !== null && semver.valid(latest) !== null && semver.gt(latest, current)
})

// 显示错误通知：使用全局 $notify（NavLayout 已挂载），避免依赖未提供的 message provider
function showErrorNotification(message: string) {
    if (typeof window !== 'undefined' && (window as any).$notify) {
        (window as any).$notify.error(message)
    } else {
        console.error(message)
    }
}

async function handleCheckUpdate() {
    if (checkingUpdate.value) return
    checkingUpdate.value = true
    try {
        const info = await musicApi.checkForUpdate()
        updateInfo.value = info
        showUpdateModal.value = true
    } catch (error) {
        console.error('检查更新失败:', error)
        showErrorNotification('检查更新失败，请稍后重试')
    } finally {
        checkingUpdate.value = false
    }
}
</script>

<style scoped>
.settings-view {
    max-width: 600px;
    padding: 16px 0;
    /* 让设置页占满父容器高度，使用 flex 列布局 */
    display: flex;
    flex-direction: column;
    min-height: 100%;
}

/* 移动端移除最大宽度限制，撑满父容器 */
.settings-view.is-narrow {
    max-width: none;
}

.settings-section {
    margin-bottom: 24px;
}

.account-section {
    margin-bottom: 32px;
}

.settings-section+.settings-section {
    border-top: 1px solid var(--border-color, #e0e0e0);
    padding-top: 24px;
}

.section-title {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--color-text);
}

/* 检查更新入口样式：与关于入口类似，居中 */
.check-update-entry {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 24px;
}

.update-status-text {
    font-size: 13px;
    color: var(--color-text-secondary);
}

.about-entry {
    /* 将关于入口推到底部 */
    margin-top: auto;
    padding-top: 24px;
    text-align: center;
}

/* 更新信息弹窗内部样式 */
.update-content {
    line-height: 1.6;
    /* 增加内容区上下空白，使弹窗不显得拥挤 */
    padding: 8px 0;
}

.version-line {
    font-size: 15px;
    font-weight: 500;
    margin-bottom: 8px;
}

.publish-date {
    font-size: 13px;
    color: var(--color-text-secondary);
    margin-bottom: 16px;
}

.update-body {
    margin-bottom: 20px;
}

.body-label {
    font-weight: 500;
}

.body-text {
    margin-top: 4px;
    /* 适配 Markdown 渲染后的 HTML 内容，取消 pre-wrap 改为正常换行 */
    color: var(--color-text);
    max-height: 300px;
    overflow-y: auto;
    line-height: 1.6;
}

/* Markdown 内容的基础样式，保证标题、列表、代码块等可读 */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
    margin: 12px 0 8px;
    font-weight: 600;
}

.markdown-body :deep(p) {
    margin: 8px 0;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
    padding-left: 24px;
    margin: 8px 0;
}

.markdown-body :deep(code) {
    background-color: var(--bg-body);
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 0.9em;
}

.markdown-body :deep(pre) {
    background-color: var(--bg-body);
    padding: 12px;
    border-radius: 6px;
    overflow-x: auto;
}

.markdown-body :deep(pre code) {
    background: none;
    padding: 0;
}

.markdown-body :deep(a) {
    color: #4098fc;
}

/* 资产列表样式 */
.assets-section {
    margin-bottom: 20px;
}

.asset-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
}

.asset-link {
    color: #4098fc;
    text-decoration: none;
    font-size: 14px;
    transition: opacity 0.2s;
}

.asset-link:hover {
    opacity: 0.8;
    text-decoration: underline;
}

.modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
}
</style>