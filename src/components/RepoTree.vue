<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { ChevronDownIcon, ChevronRightIcon, FileCode2Icon, FolderIcon, FolderOpenIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { ref } from 'vue'
import type { TreeEntry } from '@/types/git'

const props = defineProps<{ repoPath: string; entry: TreeEntry; depth?: number }>()
const children = ref<TreeEntry[]>([])
const expanded = ref(false)
const loaded = ref(false)
const error = ref('')

async function toggle() {
  if (!props.entry.isDirectory) return
  expanded.value = !expanded.value
  if (loaded.value || !expanded.value) return
  try {
    children.value = await invoke<TreeEntry[]>('get_repository_tree', {
      path: props.repoPath,
      relativePath: props.entry.path,
    })
    loaded.value = true
  } catch (reason) {
    error.value = String(reason)
  }
}
</script>

<template>
  <div>
    <Button variant="ghost"
      class="flex min-h-8 w-full justify-start items-center gap-2 rounded-md px-2 text-left text-sm text-foreground/90 transition-colors hover:bg-accent"
      :style="{ paddingLeft: `${(depth ?? 0) * 16 + 8}px` }"
      :aria-expanded="entry.isDirectory ? expanded : undefined"
      @click="toggle"
    >
      <ChevronDownIcon v-if="entry.isDirectory && expanded" class="size-3.5 text-muted-foreground" />
      <ChevronRightIcon v-else-if="entry.isDirectory" class="size-3.5 text-muted-foreground" />
      <span v-else class="w-3.5" />
      <FolderOpenIcon v-if="entry.isDirectory && expanded" class="size-4 text-primary" />
      <FolderIcon v-else-if="entry.isDirectory" class="size-4 text-primary" />
      <FileCode2Icon v-else class="size-4 text-muted-foreground" />
      <span class="truncate">{{ entry.name }}</span>
    </Button>
    <p v-if="error" class="px-3 py-1 text-xs text-destructive">{{ error }}</p>
    <div v-if="expanded && loaded">
      <RepoTree
        v-for="child in children"
        :key="child.path"
        :repo-path="repoPath"
        :entry="child"
        :depth="(depth ?? 0) + 1"
      />
      <p v-if="children.length === 0" class="py-1 text-xs text-muted-foreground" :style="{ paddingLeft: `${(depth ?? 0) * 16 + 42}px` }">空目录</p>
    </div>
  </div>
</template>
