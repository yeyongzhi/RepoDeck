import { createApp, watch } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import App from './App.vue'
import { router } from '@/router'
import { useSettingsStore } from '@/stores/settings'

import { useGroupsStore } from '@/stores/groups'
import { setActivityLimit } from '@/lib/git-client'

const pinia = createPinia()
const app = createApp(App).use(pinia).use(router)
void useSettingsStore(pinia).initialize().then(async () => {
 await useGroupsStore(pinia).initialize()
 const settings = useSettingsStore(pinia)
 watch(() => settings.options.activityLimit, setActivityLimit, { immediate: true })
 await router.isReady()
 if (router.currentRoute.value.path === '/' && settings.options.startup === 'last') {
  try { const last = localStorage.getItem('repodeck.last-repository'); if (last?.startsWith('/repositories/')) await router.replace(last) } catch {}
 }
 app.mount('#app')
})
