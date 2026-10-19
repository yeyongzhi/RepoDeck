// Git reports per-stage progress, not elapsed-time or total-byte progress.
export function cloneProgress(output: string, previous = 0): number {
  let percent = previous
  for (const line of output.split(/[\r\n]/)) {
    const match = line.match(/(\d+)%/)
    if (!match) continue
    const value = Number(match[1]) / 100
    const stage = /Receiving objects:/.test(line) ? 10 + value * 70
      : /Resolving deltas:/.test(line) ? 80 + value * 15
      : /Updating files:|Checking out files:/.test(line) ? 95 + value * 4
      : /remote:.*(?:Counting|Compressing) objects:/.test(line) ? value * 10 : 0
    percent = Math.max(percent, Math.min(99, Math.floor(stage)))
  }
  return percent
}
