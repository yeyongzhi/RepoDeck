<script setup lang="ts">
import { FolderPlusIcon, DownloadIcon, ChevronDownIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem } from '@/components/ui/dropdown-menu'
import { useCloneStore } from '@/stores/clone'
defineProps<{ disabled?: boolean; compact?: boolean; label?: string }>()
defineEmits<{ local: [] }>()
const clone = useCloneStore()
</script>
<template><DropdownMenu><DropdownMenuTrigger as-child><Button size="sm" :variant="compact ? 'ghost' : 'default'" :disabled="disabled" aria-label="添加仓库"><FolderPlusIcon data-icon="inline-start" /><template v-if="!compact">{{ label || '添加仓库' }}<ChevronDownIcon data-icon="inline-end" /></template></Button></DropdownMenuTrigger><DropdownMenuContent align="end"><DropdownMenuGroup><DropdownMenuItem @select="$emit('local')"><FolderPlusIcon />从本地添加</DropdownMenuItem><DropdownMenuItem @select="clone.openDialog()"><DownloadIcon />从远程添加</DropdownMenuItem></DropdownMenuGroup></DropdownMenuContent></DropdownMenu></template>
