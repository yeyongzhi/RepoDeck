<script setup lang="ts">
import { computed, ref } from 'vue'
import { CheckIcon, PaletteIcon } from '@lucide/vue'
import { useSettingsStore } from '@/stores/settings'
import { palettes } from '@/lib/palettes'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Input } from '@/components/ui/input'
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field'
const settings = useSettingsStore()
const opened = ref('')
const custom = ref<string>(/^#[0-9a-f]{6}$/i.test(settings.accent) ? settings.accent : '#3B82F6')
const valid = computed(() => /^#[0-9a-f]{6}$/i.test(custom.value))
const selectedGroup = computed(() => settings.accentSource === 'custom' ? undefined : palettes.find(group => group.colors.some(color => color[1].toLowerCase() === settings.accent.toLowerCase()))?.id)
const isCustom = computed(() => /^#[0-9a-f]{6}$/i.test(settings.accent) && !selectedGroup.value)
function select(color: string, customSelection = false) { if (/^#[0-9a-f]{6}$/i.test(color)) { settings.accentSource = customSelection ? 'custom' : 'palette'; settings.accent = color as `#${string}`; opened.value = '' } }
</script>
<template>
 <div class="flex flex-wrap items-center gap-2" aria-label="主题色系">
  <Button :variant="settings.accent === 'blue' ? 'outline' : 'ghost'" :aria-pressed="settings.accent === 'blue'" @click="settings.accentSource = 'palette'; settings.accent = 'blue'"><span class="flex size-5 items-center justify-center rounded-full bg-[#3B82F6] text-white"><CheckIcon v-if="settings.accent === 'blue'" /></span>品牌色</Button>
  <Popover v-for="group in palettes" :key="group.id" :open="opened === group.id" @update:open="value => opened = value ? group.id : ''">
   <PopoverTrigger as-child><Button :variant="selectedGroup === group.id ? 'outline' : 'ghost'" :aria-pressed="selectedGroup === group.id"><span v-if="selectedGroup === group.id" class="flex size-5 items-center justify-center rounded-full text-white" :style="{background: settings.accent}"><CheckIcon /></span><PaletteIcon v-else />{{ group.name }}<span v-if="selectedGroup === group.id" class="text-xs">· {{ group.colors.find(color => color[1].toLowerCase() === settings.accent.toLowerCase())?.[0] }}</span></Button></PopoverTrigger>
   <PopoverContent align="start" class="w-80"><p class="text-sm font-medium">{{ group.name }}</p><p class="mt-1 text-xs text-muted-foreground">{{ group.description }}</p><div class="mt-3 grid grid-cols-2 gap-1"><Button v-for="color in group.colors" :key="color[1]" variant="ghost" class="h-auto justify-start px-2 py-3" :aria-pressed="settings.accent.toLowerCase() === color[1].toLowerCase()" @click="select(color[1])"><span class="flex size-5 shrink-0 items-center justify-center rounded-full text-white" :style="{background: color[1]}"><CheckIcon v-if="settings.accent.toLowerCase() === color[1].toLowerCase()" /></span><span class="flex min-w-0 flex-1 flex-col items-start gap-0.5"><span>{{ color[0] }}</span><span class="text-[0.7rem] text-muted-foreground">{{ color[1] }}</span></span></Button></div></PopoverContent>
  </Popover>
  <Popover :open="opened === 'custom'" @update:open="value => opened = value ? 'custom' : ''"><PopoverTrigger as-child><Button :variant="isCustom ? 'outline' : 'ghost'" :aria-pressed="isCustom"><span v-if="isCustom" class="flex size-5 items-center justify-center rounded-full text-white" :style="{background: settings.accent}"><CheckIcon /></span><PaletteIcon v-else />自定义<span v-if="isCustom" class="font-mono text-xs">· {{ settings.accent.toUpperCase() }}</span></Button></PopoverTrigger><PopoverContent class="w-64"><Field><FieldLabel for="custom-accent">自定义主题色</FieldLabel><FieldLabel for="custom-color-picker">色盘</FieldLabel><Input id="custom-color-picker" type="color" :model-value="valid ? custom : '#3B82F6'" class="h-16 w-full cursor-pointer p-1" aria-label="选择自定义颜色" @update:model-value="value => custom = String(value)" /><Input id="custom-accent" v-model="custom" placeholder="#3B82F6" maxlength="7" :aria-invalid="!valid" /><FieldDescription>点击色盘选择颜色，或输入六位十六进制颜色值</FieldDescription><Button :disabled="!valid" @click="select(custom, true)"><span class="size-4 rounded-full" :style="{background: valid ? custom : 'transparent'}" />应用颜色</Button></Field></PopoverContent></Popover>
 </div>
</template>
