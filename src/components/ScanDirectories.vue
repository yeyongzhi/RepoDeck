<script setup lang="ts">
import { computed, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { useWorkspaceStore } from '@/stores/workspace'
import { displayPath } from '@/lib/path'
import { Button } from '@/components/ui/button'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Item, ItemGroup, ItemContent, ItemTitle, ItemActions } from '@/components/ui/item'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
const workspace = useWorkspaceStore()
const directories = computed(() => workspace.roots.filter(item => item.kind !== 'repository'))
const pending = ref<string | null>(null)
const saving = ref(false)
const disabled = computed(() => workspace.isLoading || workspace.operationBusy || saving.value)
async function add() {
 if (!isTauri()) return toast.info('请在桌面应用中添加扫描目录')
 try {
  const path = await open({directory:true,multiple:false,title:'选择固定扫描目录'})
  if (typeof path !== 'string') return
  await workspace.addRoot(path, 'workspace')
  if (workspace.error) throw new Error(workspace.error)
  toast.success('扫描目录已保存，仓库已更新')
 } catch (reason) { toast.error(String(reason)) }
}
async function scan(path?: string) {
 await workspace.refresh(true, path)
 if (workspace.error) toast.error(workspace.error)
 else toast.success(workspace.issues.length ? '扫描完成，部分目录无法访问，请在首页查看详情' : '扫描完成')
}
async function remove() {
 if (!pending.value) return
 saving.value = true
 try { await workspace.removeRoot(pending.value); pending.value = null; toast.success('已停止扫描此目录，仓库和分组保留') } catch (reason) { toast.error(String(reason)) }
 finally { saving.value = false }
}
</script>
<template><Card class="gap-4 py-4"><CardHeader class="px-4"><CardTitle>固定扫描目录</CardTitle><CardDescription>递归发现目录中的 Git 仓库，自动去重并保留分组关系。移除目录只停止扫描。</CardDescription></CardHeader><CardContent class="flex flex-col gap-4 px-4"><div class="flex flex-wrap gap-2"><Button size="sm" :disabled="disabled" @click="add">添加扫描目录</Button><Button size="sm" variant="outline" :disabled="disabled || !directories.length" @click="scan()">{{ workspace.isLoading ? '扫描中…' : '扫描全部目录' }}</Button></div><ItemGroup class="gap-2"><Item v-for="directory in directories" :key="directory.path" variant="outline" size="sm"><ItemContent class="min-w-0"><ItemTitle class="break-all">{{ displayPath(directory.path) }}</ItemTitle></ItemContent><ItemActions><Button size="xs" variant="outline" :disabled="disabled" @click="scan(directory.path)">立即扫描</Button><Button size="xs" variant="destructive-ghost" :disabled="disabled" @click="pending = directory.path">移除</Button></ItemActions></Item></ItemGroup><p v-if="!directories.length" class="text-sm text-muted-foreground">暂无固定扫描目录，添加后会立即扫描。</p></CardContent></Card><Dialog :open="pending !== null" @update:open="value => { if (!value && !saving) pending = null }"><DialogContent><DialogHeader><DialogTitle>移除扫描目录？</DialogTitle><DialogDescription class="break-all">{{ pending }}。以后不再自动扫描此目录；已登记仓库、分组和磁盘文件均保留。</DialogDescription></DialogHeader><DialogFooter><Button variant="outline" :disabled="saving" @click="pending = null">取消</Button><Button :disabled="saving" @click="remove">确认移除</Button></DialogFooter></DialogContent></Dialog></template>
