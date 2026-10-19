import { createApp, watch } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import App from './App.vue'
import { router } from '@/router'
import { useSettingsStore } from '@/stores/settings'

import { useGroupsStore } from '@/stores/groups'
import { initializeBusiness, business } from '@/lib/business'
import { initializeActivities, setActivityLimit } from '@/lib/git-client'

const pinia = createPinia()
const app = createApp(App).use(pinia).use(router)
void useSettingsStore(pinia).initialize().then(async () => {
 await initializeBusiness()
 await initializeActivities()
 await useGroupsStore(pinia).initialize()
 const settings = useSettingsStore(pinia)
 watch(() => settings.options.activityLimit, setActivityLimit, { immediate: true })
 await router.isReady()
 if (router.currentRoute.value.path === '/' && settings.options.startup === 'last') {
  try { const last = business.last; if (last?.startsWith('/repositories/')) await router.replace(last) } catch {}
 }
 app.mount('#app')
}).catch(error => { document.getElementById('app')!.textContent = `本地数据读取失败，已保留原数据。请检查安装目录权限后重启：${String(error)}` })
