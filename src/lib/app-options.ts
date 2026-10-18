export const defaultAppOptions = {
  startupScan: 'on', startup: 'last', repositoryLayout: 'remember', repositoryTab: 'overview',
  dateFormat: 'full', density: 'standard', repositorySort: 'name', activityLimit: 300,
}
export type AppOptions = typeof defaultAppOptions
export function normalizeAppOptions(value: unknown): AppOptions {
  const raw = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  const result = { ...defaultAppOptions }
  const choices = {
    startupScan: ['on', 'off'], startup: ['last', 'home'], repositoryLayout: ['remember', 'grid', 'list'],
    repositoryTab: ['overview', 'branches', 'commits', 'worktree', 'compare', 'stash', 'tags', 'activity'],
    dateFormat: ['full', 'relative'], density: ['compact', 'standard', 'comfortable'],
    repositorySort: ['name', 'recent', 'status'],
  }
  for (const key of Object.keys(choices) as (keyof typeof choices)[]) {
    const candidate = raw[key]
    if (typeof candidate === 'string' && choices[key].includes(candidate)) result[key] = candidate
  }
  if ([100, 300, 500, 1000].includes(Number(raw.activityLimit))) result.activityLimit = Number(raw.activityLimit)
  return result
}
export function formatAppDate(value: string | number, mode: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  if (mode === 'relative') {
    const seconds = (date.getTime() - Date.now()) / 1000
    const formatter = new Intl.RelativeTimeFormat('zh-CN', { numeric: 'auto' })
    for (const [unit, size] of [['year', 31536000], ['month', 2592000], ['day', 86400], ['hour', 3600], ['minute', 60]] as const) {
      if (Math.abs(seconds) >= size) return formatter.format(Math.round(seconds / size), unit)
    }
    return formatter.format(Math.round(seconds), 'second')
  }
  return new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }).format(date)
}
