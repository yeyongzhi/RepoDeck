export function remoteWebUrl(remote: string): string | null {
  let value = remote.trim()
  const scp = /^(?:[^@/]+@)?([^/:]+):(.+)$/.exec(value)
  if (!value.includes('://') && scp) value = `https://${scp[1]}/${scp[2]}`
  try {
    const url = new URL(value.replace(/^ssh:\/\//i, 'https://'))
    if (!['http:', 'https:'].includes(url.protocol)) return null
    url.username = ''; url.password = ''; url.search = ''; url.hash = ''
    url.pathname = url.pathname.replace(/\.git\/?$/, '').replace(/\/$/, '')
    return url.toString().replace(/\/$/, '')
  } catch { return null }
}
export function remoteBranchUrl(remote: string, branch: string | null): string | null {
  const base = remoteWebUrl(remote)
  if (!base || !branch) return null
  const host = new URL(base).hostname.toLowerCase()
  const encoded = encodeURIComponent(branch)
  if (host === 'github.com' || host === 'gitee.com') return `${base}/tree/${encoded}`
  if (host === 'gitlab.com') return `${base}/-/tree/${encoded}`
  if (host === 'bitbucket.org') return `${base}/src/${encoded}`
  return null
}
