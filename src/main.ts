import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import App from './App.vue'
import { router } from '@/router'
import { useSettingsStore } from '@/stores/settings'

const pinia = createPinia()
const app = createApp(App).use(pinia).use(router)
void useSettingsStore(pinia).initialize().then(() => app.mount('#app'))
