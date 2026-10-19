import { reactive } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'
export const business = reactive({ pins: [] as string[], visits: {} as Record<string, number>, last: '' })
let ready = false
let queue: Promise<unknown> = Promise.resolve()
export const businessValues: Record<string, unknown> = {}
export async function initializeBusiness() {
  const legacy: Record<string, unknown> = {}
  for (const [key, old] of Object.entries({activities:'activity', clones:'clones', pins:'pins', visits:'recent-repositories', last:'last-repository', directories:'cloneDirectories', groups:'groups'})) {
    try { const value = localStorage.getItem(`repodeck.${old}`); if (value) legacy[key] = key === 'last' ? value : JSON.parse(value) } catch { /* Keep malformed legacy data untouched. */ }
  }
  if (isTauri()) await invoke('migrate_browser_data', {values: legacy})
  for (const key of ['pins','visits','last','activities','clones']) businessValues[key] = isTauri() ? await invoke('read_business_data',{key}) : legacy[key]
  business.pins = (businessValues.pins as string[]) ?? []
  business.visits = (businessValues.visits as Record<string,number>) ?? {}
  business.last = (businessValues.last as string) ?? ''
  ready = true
}
export function saveBusiness(key: string, value: unknown): Promise<unknown> {
  const snapshot = JSON.parse(JSON.stringify(value))
  if (!ready) return Promise.reject(new Error('业务数据尚未读取，已阻止覆盖本地数据'))
  businessValues[key] = snapshot
  const work = queue.catch(() => {}).then(() => isTauri() ? invoke('save_business_data',{key,value:snapshot}) : localStorage.setItem(`repodeck.${key}`,JSON.stringify(snapshot)))
  queue = work
  return work
}
export function persistBusiness(key: string, value: unknown) { void saveBusiness(key,value).catch(e => toast.error(String(e))) }
export function flushBusiness() { return queue }
export function togglePin(id: string) { business.pins = business.pins.includes(id) ? business.pins.filter(p => p !== id) : [...business.pins,id]; persistBusiness('pins',business.pins) }
export function visit(id: string) { business.visits[id] = Date.now(); business.last = `/repositories/${encodeURIComponent(id)}`; persistBusiness('visits', business.visits); persistBusiness('last',business.last) }
