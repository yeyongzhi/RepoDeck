<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDownToLineIcon, ArrowUpFromLineIcon, ChevronRightIcon, CopyIcon, EllipsisIcon, FolderGit2Icon, GitBranchIcon, GitCommitHorizontalIcon, RefreshCwIcon, PinIcon } from '@lucide/vue'
import type { RepositorySummary } from '@/types/git'
import { displayPath } from '@/lib/path'
import { actionReason } from '@/lib/repository-rules'
import AppTooltip from '@/components/AppTooltip.vue'
import { formatAppDate } from '@/lib/app-options'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'

const props = defineProps<{ repo: RepositorySummary; layout: 'grid' | 'list'; checked: boolean; disabled: boolean; pinned?: boolean }>()
const emit = defineEmits<{ select: []; pin: []; action: [action: 'checkout' | 'fetch' | 'commit' | 'pull' | 'push' | 'copy' | 'groups'] }>()
const detailsLink = computed(() => ({ name: 'repository', params: { id: props.repo.id } }))
const state = computed(() => {
  const repo = props.repo
  if (repo.hasConflicts) return { label: '存在冲突', variant: 'warning' as const }
  if (repo.operationInProgress) return { label: '操作待继续', variant: 'warning' as const }
  if (repo.isDirty) return { label: '有未提交改动', variant: 'warning' as const }
  if (repo.isDetached) return { label: 'Detached HEAD', variant: 'warning' as const }
  if (!repo.upstream) return { label: '未设置跟踪', variant: 'outline' as const }
  if (repo.ahead && repo.behind) return { label: '分支已分叉', variant: 'warning' as const }
  if (repo.behind) return { label: `落后 ${repo.behind} 个提交`, variant: 'info' as const }
  if (repo.ahead) return { label: `待推送 ${repo.ahead} 个提交`, variant: 'info' as const }
  return { label: '引用一致', variant: 'success' as const }
})
const clean = computed(() => !props.repo.isDirty && !props.repo.isDetached && !props.repo.hasConflicts && !props.repo.operationInProgress)
</script>

<template>
  <Card :class="cn('repository-card relative min-w-0 gap-0 overflow-hidden py-0 transition-shadow hover:shadow-md', checked && 'ring-2 ring-primary/30')" :data-layout="layout">
    <div class="absolute right-3 top-3 flex items-center gap-2">
<AppTooltip :text="pinned ? '取消置顶' : '置顶仓库'"><Button variant="ghost" size="icon-sm" :aria-label="pinned ? '取消置顶' : '置顶仓库'" :aria-pressed="pinned" @click="emit('pin')" :title="pinned ? '取消置顶' : '置顶仓库'"><PinIcon :class="pinned && 'text-primary'" /></Button></AppTooltip><Checkbox :model-value="checked" :aria-label="`选择 ${repo.name} 用于批量同步`" :disabled="disabled" @update:model-value="emit('select')" />
      <DropdownMenu>
        <DropdownMenuTrigger as-child><Button variant="ghost" size="icon-sm" :aria-label="`${repo.name} 功能菜单`" :disabled="disabled" :title="`${repo.name} 功能菜单`"><EllipsisIcon /></Button></DropdownMenuTrigger>
        <DropdownMenuContent align="end" class="w-52">
          <DropdownMenuLabel class="truncate">{{ repo.name }}</DropdownMenuLabel>
          <DropdownMenuGroup>
            <DropdownMenuItem as-child><RouterLink :to="detailsLink"><FolderGit2Icon />查看仓库详情</RouterLink></DropdownMenuItem>
            <DropdownMenuItem @select="emit('action', 'checkout')"><GitBranchIcon />切换分支</DropdownMenuItem>
            <DropdownMenuItem @select="emit('action', 'fetch')"><RefreshCwIcon />Fetch 获取远端更新</DropdownMenuItem>
            <DropdownMenuItem :disabled="!repo.isDirty || repo.hasConflicts || repo.operationInProgress" @select="emit('action', 'commit')"><GitCommitHorizontalIcon /><span>审阅并提交改动<span v-if="actionReason(repo,'commit')" class="block text-xs">{{ actionReason(repo,'commit') }}</span></span></DropdownMenuItem>
            <DropdownMenuItem :disabled="!clean || !repo.upstream || repo.ahead !== 0 || repo.behind === 0" @select="emit('action', 'pull')"><ArrowDownToLineIcon /><span>快进拉取<span v-if="actionReason(repo,'pull')" class="block text-xs">{{ actionReason(repo,'pull') }}</span></span></DropdownMenuItem>
            <DropdownMenuItem :disabled="!clean || !repo.upstream || repo.ahead === 0 || repo.behind !== 0" @select="emit('action', 'push')"><ArrowUpFromLineIcon /><span>推送当前分支<span v-if="actionReason(repo,'push')" class="block text-xs">{{ actionReason(repo,'push') }}</span></span></DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup><DropdownMenuItem @select="emit('action', 'groups')"><FolderGit2Icon />添加到分组</DropdownMenuItem><DropdownMenuItem @select="emit('action', 'copy')"><CopyIcon />复制仓库路径</DropdownMenuItem></DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
    <RouterLink :to="detailsLink" :aria-label="`查看 ${repo.name} 仓库详情`" class="repository-card-link rounded-xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring">
      <CardHeader class="repository-card-heading gap-3 px-5 pb-3 pt-5">
        <span class="flex size-10 items-center justify-center rounded-xl bg-accent text-primary"><FolderGit2Icon class="size-5" /></span>
        <div class="min-w-0"><CardTitle class="truncate pr-28" :title="repo.name">{{ repo.name }}</CardTitle><CardDescription class="mt-2 truncate" :title="displayPath(repo.path)">{{ displayPath(repo.path) }}</CardDescription></div>
      </CardHeader>
      <CardContent class="repository-card-status flex min-w-0 flex-wrap items-center gap-2 px-5 pb-4"><Badge variant="secondary" class="max-w-full"><GitBranchIcon /><span class="truncate">{{ repo.currentBranch ?? (repo.isDetached ? 'Detached HEAD' : '无分支') }}</span></Badge><Badge :variant="state.variant">{{ state.label }}</Badge></CardContent>
      <CardFooter class="repository-card-footer flex min-w-0 justify-between gap-3 border-t px-5 py-3">
        <span class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground"><span>{{ repo.changedFiles === undefined ? (repo.isDirty ? '有未提交改动' : '工作区干净') : `${repo.changedFiles} 个未提交文件` }}</span><span class="tabular-nums" title="当前分支领先 / 落后跟踪分支的提交数量">↑ {{ repo.ahead }} / ↓ {{ repo.behind }}</span></span><ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
      </CardFooter><p class="px-5 pb-3 text-xs text-muted-foreground" :title="repo.lastFetched ? formatAppDate(repo.lastFetched * 1000, 'full') : '点击 Fetch 获取远程最新状态'">{{ repo.lastFetched ? `最近获取远端：${formatAppDate(repo.lastFetched * 1000, 'relative')}` : '尚未获取远端，当前为本地引用状态' }}<span v-if="repo.lastFetched && Date.now()/1000 - repo.lastFetched > 3600"> · 状态可能已变化</span></p>
    </RouterLink>
  </Card>
</template>
