export function compareVersions(left: string, right: string): number {
 const parse = (value: string) => /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/.exec(value)
 const a = parse(left), b = parse(right)
 if (!a || !b) throw new Error('无法识别版本号，请到项目主页查看发布版本')
 for (let i = 1; i <= 3; i++) {
  const difference = Number(a[i]) - Number(b[i])
  if (difference) return difference
 }
 if (a[4] === b[4]) return 0
 if (!a[4]) return 1
 if (!b[4]) return -1
 // Split numeric suffixes too: existing releases use beta1, beta2, beta10.
 const parts = (value: string) => value.match(/\d+|[^\d.]+|\./g) ?? []
 const ap = parts(a[4]), bp = parts(b[4])
 for (let i = 0; i < Math.max(ap.length, bp.length); i++) {
  const x = ap[i], y = bp[i]
  if (x === undefined) return -1
  if (y === undefined) return 1
  if (x === y) continue
  const xn = /^\d+$/.test(x), yn = /^\d+$/.test(y)
  if (xn && yn) return Number(x) - Number(y)
  if (xn !== yn) return xn ? -1 : 1
  return x < y ? -1 : 1
 }
 return 0
}

export function newestRelease(releases: unknown): string | undefined {
 if (!Array.isArray(releases)) throw new Error('版本信息格式异常，请稍后重试')
 let latest: string | undefined
 for (const release of releases) {
  if (!release || release.draft || typeof release.tag_name !== 'string') continue
  try {
   if (compareVersions(release.tag_name, latest ?? '0.0.0-0') > 0) latest = release.tag_name
  } catch { /* Ignore tags that are not application versions. */ }
 }
 return latest
}
