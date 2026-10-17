export type ThemeMode = 'light' | 'dark' | 'system'
export type AccentColor = 'blue' | 'violet' | 'green' | 'neutral' | `#${string}`
export interface Preferences {
  theme: ThemeMode
  accent: AccentColor
  commitLimit: number
}

export const defaultPreferences: Preferences = { theme: 'system', accent: 'blue', commitLimit: 10 }

export function normalizeCommitLimit(value: unknown): number {
  const number = Number(value)
  return Number.isFinite(number) ? Math.min(100, Math.max(1, Math.floor(number))) : 10
}

export function readPreferences(raw: string | null, legacyCommitLimit: string | null = null): Preferences {
  let parsed: Partial<Preferences> = {}
  try {
    const value: unknown = JSON.parse(raw ?? '{}')
    if (value && typeof value === 'object' && !Array.isArray(value)) parsed = value as Partial<Preferences>
  } catch { /* A damaged preference file must not prevent startup. */ }
  return {
    theme: ['light', 'dark', 'system'].includes(parsed.theme ?? '') ? parsed.theme! : defaultPreferences.theme,
    accent: (['blue', 'violet', 'green', 'neutral'].includes(parsed.accent ?? '') || /^#[0-9a-f]{6}$/i.test(parsed.accent ?? '')) ? parsed.accent! : defaultPreferences.accent,
    commitLimit: normalizeCommitLimit(parsed.commitLimit ?? legacyCommitLimit ?? defaultPreferences.commitLimit),
  }
}
