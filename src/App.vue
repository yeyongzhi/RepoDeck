<script setup lang="ts">
import { Toaster } from 'vue-sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import AppShell from '@/components/AppShell.vue'
import { useSettingsStore } from '@/stores/settings'
const settings = useSettingsStore()
</script>

<template>
  <TooltipProvider :delay-duration="300"><AppShell><RouterView v-slot="{ Component }"><KeepAlive :max="2"><component :is="Component" /></KeepAlive></RouterView></AppShell></TooltipProvider>
  <Toaster :theme="settings.isDark ? 'dark' : 'light'" position="bottom-right" :duration="3500" close-button />
</template>
