<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { useSettingsStore } from '@/stores/settings'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Field, FieldGroup, FieldLabel, FieldDescription } from '@/components/ui/field'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
const settings = useSettingsStore()
async function select(kind: 'editorPath' | 'terminalPath') {
  try { const path = await open({multiple:false,title:'选择工具可执行文件',filters:[{name:'可执行程序',extensions:['exe']}]}); if (typeof path === 'string') settings[kind] = path } catch(e) { toast.error(String(e)) }
}
</script>
<template><Card><CardHeader><CardTitle>开发工具</CardTitle><CardDescription>选择默认编辑器和终端，修改自动保存到偏好 JSON。</CardDescription></CardHeader><CardContent><FieldGroup><Field v-for="item in ([{key:'editorPath',label:'默认编辑器',description:'留空时使用 VS Code；选择编辑器的可执行文件。'},{key:'terminalPath',label:'默认终端',description:'留空时使用系统终端；启动目录为当前仓库。'}] as const)" :key="item.key"><FieldLabel :for="item.key">{{ item.label }}</FieldLabel><div class="flex flex-wrap gap-2"><Input :id="item.key" v-model="settings[item.key]" class="min-w-0 flex-1" placeholder="使用默认工具" /><Button variant="outline" @click="select(item.key)">选择程序</Button><Button variant="ghost" @click="settings[item.key] = ''">恢复默认</Button></div><FieldDescription>{{ item.description }}</FieldDescription></Field></FieldGroup></CardContent></Card></template>
