<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { useGroupsStore } from '@/stores/groups'
import { useWorkspaceStore } from '@/stores/workspace'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip'
import { displayPath } from '@/lib/path'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
const groups = useGroupsStore()
const workspace = useWorkspaceStore()
const router = useRouter()
const legacyColors: Record<string, string> = {primary:'#3B82F6',success:'#22C55E',warning:'#F59E0B',info:'#0EA5E9',muted:'#94A3B8'}
const name = ref(''); const color = ref('primary'); const search = ref(''); const selected = ref<string[]>([]); const error = ref(''); const deleting = ref(false)
const editingGroup = computed(() => groups.groups.find(item => item.id === groups.editing))
const choices = computed(() => (groups.repository || groups.repositories.length) ? groups.groups.map(item => ({id:item.id,name:item.name,description:`${item.members.length} 个仓库`})) : workspace.repositories.map(item => ({id:item.id,name:item.name,description:displayPath(item.path)})))
const filtered = computed(() => choices.value.filter(item => `${item.name} ${item.description}`.toLowerCase().includes(search.value.trim().toLowerCase())))
watch(() => groups.dialogOpen, open => {
 if (!open) return
 const item = editingGroup.value; name.value = item?.name ?? ''; color.value = item ? legacyColors[item.color] ?? item.color : '#3B82F6'; search.value = ''; error.value = ''; deleting.value = false
 selected.value = groups.repositories.length ? [] : groups.repository ? groups.groups.filter(item => item.members.includes(groups.repository!)).map(item => item.id) : [...(item?.members ?? [])]
})
function toggle(id: string) { selected.value = selected.value.includes(id) ? selected.value.filter(item => item !== id) : [...selected.value,id] }
async function save() {
 try {
  if (groups.repositories.length) {
   await groups.save(groups.groups.map(item => selected.value.includes(item.id) ? {...item,members:[...new Set([...item.members,...groups.repositories])]} : item))
  } else if (groups.repository) {
   const id = groups.repository
   await groups.save(groups.groups.map(item => ({...item,members: selected.value.includes(item.id) ? [...new Set([...item.members,id])] : item.members.filter(member => member !== id)})))
  } else {
   if (!/^#[0-9a-f]{6}$/i.test(color.value)) throw new Error('请选择有效颜色或输入六位十六进制颜色值')
   const trimmed = name.value.trim()
   if (!trimmed || trimmed.length > 50) throw new Error('请输入 1–50 个字符的分组名称')
   if (groups.groups.some(item => item.id !== groups.editing && item.name.toLocaleLowerCase() === trimmed.toLocaleLowerCase())) throw new Error('分组名称已存在')
   const item = {id:groups.editing ?? crypto.randomUUID(),name:trimmed,color:color.value,members:[...new Set(selected.value)]}
   await groups.save(groups.editing ? groups.groups.map(group => group.id === item.id ? item : group) : [...groups.groups,item])
  }
  groups.dialogOpen = false; toast.success('分组已保存')
 } catch (reason) { error.value = String(reason); toast.error(error.value) }
}
async function remove() {
 try { const id = groups.editing; await groups.save(groups.groups.filter(item => item.id !== id)); groups.dialogOpen = false; if (router.currentRoute.value.query.group === id) await router.push('/'); toast.success('分组已删除') } catch (reason) { error.value = String(reason); toast.error(error.value) }
}
</script>
<template>
 <Dialog v-model:open="groups.dialogOpen"><DialogContent class="flex max-h-[85dvh] flex-col"><DialogHeader><DialogTitle>{{ deleting ? '删除分组？' : (groups.repository || groups.repositories.length) ? '添加到分组' : groups.editing ? '管理分组' : '新建分组' }}</DialogTitle><DialogDescription>{{ deleting ? '仅删除分组，仓库登记、磁盘文件和 Git 历史均保留。' : '同一个仓库可加入多个分组，调整成员只改变分组关系。' }}</DialogDescription></DialogHeader><template v-if="!deleting"><FieldGroup v-if="!groups.repository && !groups.repositories.length"><Field><FieldLabel for="group-name">分组名称</FieldLabel><Input id="group-name" v-model="name" maxlength="50" placeholder="例如：我的产品" /></Field><Field><FieldLabel for="group-color">标识颜色</FieldLabel><div class="flex items-center gap-3"><Input id="group-color" v-model="color" type="color" class="h-10 w-14 shrink-0 cursor-pointer p-1" aria-label="选择分组颜色" /><Input v-model="color" class="h-10 min-w-0 flex-1" maxlength="7" placeholder="#3B82F6" aria-label="分组颜色值" :aria-invalid="!/^#[0-9a-f]{6}$/i.test(color)" /></div></Field></FieldGroup><Input v-model="search" class="h-10 shrink-0" aria-label="搜索分组或仓库" placeholder="搜索名称或路径" /><ScrollArea class="h-64 min-h-0 shrink"><div class="flex flex-wrap items-start gap-x-4 gap-y-2 pr-2"><Field v-for="item in filtered" :key="item.id" orientation="horizontal" class="w-fit max-w-full flex-none justify-start gap-2 rounded-md px-2 py-2 [&>[data-slot=field-label]]:flex-none"><Checkbox :id="`member-${item.id}`" :model-value="selected.includes(item.id)" @update:model-value="toggle(item.id)" /><Tooltip><TooltipTrigger as-child><FieldLabel :for="`member-${item.id}`" class="min-w-0 max-w-64 cursor-pointer text-left"><span class="truncate">{{ item.name }}</span></FieldLabel></TooltipTrigger><TooltipContent side="bottom" align="start" :side-offset="6" class="max-w-72 break-all">{{ item.description }}</TooltipContent></Tooltip></Field><p v-if="!filtered.length" class="text-sm text-muted-foreground">{{ search ? '没有匹配结果' : groups.repository ? '暂无分组，请先在侧栏创建分组' : '暂无已登记仓库' }}</p></div></ScrollArea><p class="text-xs text-muted-foreground">已选择 {{ selected.length }} 项</p></template><p v-if="error" class="text-sm text-destructive">{{ error }}</p><DialogFooter><Button v-if="groups.editing && !deleting" variant="destructive-ghost" class="mr-auto" :disabled="groups.saving" @click="deleting = true">删除分组</Button><Button variant="outline" :disabled="groups.saving" @click="groups.dialogOpen = false">取消</Button><Button :variant="deleting ? 'destructive' : 'default'" :disabled="groups.saving" @click="deleting ? remove() : save()">{{ groups.saving ? '保存中…' : deleting ? '确认删除' : '保存' }}</Button></DialogFooter></DialogContent></Dialog>
</template>
