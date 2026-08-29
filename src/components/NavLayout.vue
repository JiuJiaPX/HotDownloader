<template>
    <div class="nav-layout" :class="{ 'is-narrow': isNarrow }">
        <!-- 宽屏左侧垂直导航 -->
        <aside v-if="!isNarrow" class="sidebar">
            <n-menu :value="currentRoute" :options="menuOptions" @update:value="handleMenuClick" />
        </aside>

        <!-- 内容区域 -->
        <main ref="mainContentRef" class="main-content" :class="{ 'has-bottom-nav': isNarrow }">
            <router-view v-slot="{ Component }">
                <keep-alive>
                    <component :is="Component" />
                </keep-alive>
            </router-view>
        </main>

        <!-- 窄屏底部水平导航（固定底部，永远居中） -->
        <footer v-if="isNarrow" class="bottom-nav">
            <div class="bottom-nav-inner">
                <n-menu :value="currentRoute" :options="menuOptions" mode="horizontal"
                    @update:value="handleMenuClick" />
            </div>
        </footer>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NMenu, useNotification, type MenuOption } from 'naive-ui'
import { useCloseGuard } from '../composables/useCloseGuard'

const router = useRouter()
const route = useRoute()

// 保存各路由页面的滚动位置，实现独立滚动记录
const mainContentRef = ref<HTMLElement | null>(null)
const scrollPositions: Record<string, number> = {}

let removeRouteGuard: (() => void) | null = null

// 恢复指定路由的滚动位置
async function restoreScrollPosition(path: string) {
    await nextTick()
    if (mainContentRef.value) {
        mainContentRef.value.scrollTop = scrollPositions[path] ?? 0
    }
}

// 监听路由变化，恢复新路由的滚动位置
watch(() => route.path, (newPath) => {
    restoreScrollPosition(newPath)
})

// 在 n-dialog-provider 内部调用，确保 useDialog 正常工作
useCloseGuard()

// 挂载通知实例到全局，供 store 使用
const notification = useNotification();
(window as any).$notify = notification

const isNarrow = ref(false)

let mediaQuery: MediaQueryList | null = null

function updateNarrow(e: MediaQueryListEvent | MediaQueryList) {
    isNarrow.value = e.matches
}

onMounted(() => {
    mediaQuery = window.matchMedia('(max-width: 767px)')
    updateNarrow(mediaQuery)
    mediaQuery.addEventListener('change', updateNarrow)

    // 注册全局前置守卫，在离开当前路由前保存滚动位置
    removeRouteGuard = router.beforeEach((_to, from, next) => {
        if (mainContentRef.value) {
            scrollPositions[from.path] = mainContentRef.value.scrollTop
        }
        next()
    })
    // 初始恢复当前路由的滚动位置（如果有保存过）
    restoreScrollPosition(route.path)
})

onUnmounted(() => {
    if (mediaQuery) {
        mediaQuery.removeEventListener('change', updateNarrow)
    }
    // 移除路由守卫，避免内存泄漏
    if (removeRouteGuard) {
        removeRouteGuard()
        removeRouteGuard = null
    }
})

const currentRoute = computed(() => route.path)

const menuOptions: MenuOption[] = [
    {
        label: '搜索',
        key: '/search'
    },
    {
        label: '歌单',
        key: '/playlist'
    },
    {
        label: '任务',
        key: '/task'
    },
    {
        label: '设置',
        key: '/settings'
    },
]

function handleMenuClick(key: string) {
    if (key !== route.path) {
        router.push(key)
    }
}
</script>

<style scoped>
/* 布局整体 */
.nav-layout {
    display: flex;
    height: 100%;
}

.nav-layout.is-narrow {
    flex-direction: column;
}

/* 侧边栏：使用自定义背景变量 */
.sidebar {
    width: 160px;
    flex-shrink: 0;
    border-right: 1px solid var(--border-color);
    padding: 12px 0;
    background-color: var(--bg-sidebar);
}

/* 主内容区背景 */
.main-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    background-color: var(--bg-content);
    /* 将回弹限制在当前滚动容器内部，保留视觉回弹但阻断滚动链向上传播，恢复主内容区滚动到顶端/底端时的回弹效果，同时避免回弹传播导致底部导航移动 */
    overscroll-behavior: contain;
}

/* 为正常流底部导航保留合适的底部间距，避免内容与导航粘连 */
.main-content.has-bottom-nav {
    padding-bottom: 16px;
}

/* 底部导航：改用正常流布局（非 fixed），解决 Android 滚动回弹时导航被拉伸的问题 */
/* 需配合 flex 列容器使用，主内容无需预留底部内边距，高度由 flex 分配 */
.bottom-nav {
    width: 100%;
    height: 56px;
    flex-shrink: 0;
    border-top: 1px solid var(--border-color);
    background-color: var(--bg-bottom);
}

/* 居中容器 */
.bottom-nav-inner {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}

/* 穿透样式强制菜单项居中 */
.bottom-nav-inner :deep(.n-menu) {
    justify-content: center;
}

.bottom-nav-inner :deep(.n-menu .n-menu-item) {
    flex: none;
}
</style>