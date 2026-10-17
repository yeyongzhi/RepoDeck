<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import type { ChangedFile } from '@/types/git'

const props = defineProps<{ change: ChangedFile }>()
const labels: Record<string, string> = {
  ' ': '无改动', M: '已修改', A: '新增', D: '已删除',
  R: '已重命名', C: '已复制', T: '类型变更', U: '冲突',
}
const status = computed(() => {
  const change = props.change
  if (change.conflicted) return { label: '冲突', description: '请先解决文件冲突' }
  if (change.untracked || change.status === '??') {
    return { label: '未跟踪', description: '新文件尚未纳入 Git；选中后可暂存并提交' }
  }
  const index = change.status[0] ?? ' '
  const worktree = change.status[1] ?? ' '
  const changed = [...new Set([index, worktree].filter((code) => code !== ' '))]
  return {
    label: changed.map((code) => labels[code] ?? '已变更').join(' / ') || '已变更',
    description: `暂存区：${labels[index] ?? '已变更'}；工作区：${labels[worktree] ?? '已变更'}`,
  }
})
</script>

<template>
  <Badge :variant="change.conflicted ? 'destructive' : change.untracked ? 'info' : 'outline'" :title="status.description">
    {{ status.label }}
  </Badge>
</template>
