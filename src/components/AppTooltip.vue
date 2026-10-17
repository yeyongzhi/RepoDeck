<script setup lang="ts">
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
defineProps<{ text: string }>()
</script>
<template><Tooltip><TooltipTrigger as-child><slot /></TooltipTrigger><TooltipContent side="right" class="max-w-80 break-all">{{ text }}</TooltipContent></Tooltip></template>
