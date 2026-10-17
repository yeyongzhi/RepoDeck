// Keep the original path for native calls; format only labels and clipboard text.
export function displayPath(path: string): string {
  if (!path.startsWith('\\\\?\\')) return path
  const suffix = path.slice(4)
  if (/^UNC\\/i.test(suffix)) return '\\\\' + suffix.slice(4)
  if (/^[a-z]:\\/i.test(suffix)) return suffix
  return path
}
