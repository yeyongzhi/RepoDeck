<script setup lang="ts">
import { computed, ref } from 'vue'
import { parseDate } from '@internationalized/date'
import type { DateValue } from 'reka-ui'
import { CalendarDaysIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Calendar } from '@/components/ui/calendar'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
const model = defineModel<string>({ default: '' })
const props = defineProps<{ placeholder: string; min?: string; max?: string }>()
const open = ref(false)
const value = computed(() => model.value ? parseDate(model.value) : undefined)
function select(date: DateValue | undefined) {
 model.value = date?.toString() ?? ''
 open.value = false
}
</script>
<template>
 <div><Popover v-model:open="open">
  <PopoverTrigger as-child><Button variant="outline" class="w-full min-w-0 justify-start" :aria-label="placeholder"><CalendarDaysIcon data-icon="inline-start" /><span class="truncate">{{ model || placeholder }}</span></Button></PopoverTrigger>
  <PopoverContent class="w-auto p-0" align="start"><Calendar :model-value="value" :min-value="props.min ? parseDate(props.min) : undefined" :max-value="props.max ? parseDate(props.max) : undefined" locale="zh-CN" layout="month-and-year" initial-focus @update:model-value="select" /></PopoverContent>
 </Popover></div>
</template>
