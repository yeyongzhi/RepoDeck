export interface ScanRoot {
  kind?: 'workspace' | 'repository'
  path: string
}

export interface AddRootResult {
  roots: ScanRoot[]
  added: boolean
}

export interface RemoteSummary {
  name: string
  url: string
}

export interface RepositorySummary {
  id: string
  name: string
  path: string
  scanRoot: string
  currentBranch: string | null
  isDetached: boolean
  upstream: string | null
  remotes: RemoteSummary[]
  localBranches: string[]
  remoteBranches: string[]
  ahead: number
  behind: number
  isDirty: boolean
  hasConflicts: boolean
  operationInProgress: boolean
}

export interface ScanIssue {
  path: string
  message: string
}

export interface ScanSnapshot {
  roots: ScanRoot[]
  repositories: RepositorySummary[]
  issues: ScanIssue[]
}

export interface TreeEntry {
  name: string
  path: string
  isDirectory: boolean
}

export interface BranchCommit {
  hash: string
  shortHash: string
  subject: string
  author: string
  committedAt: string
}

export interface ChangedFile {
  path: string
  status: string
  staged: boolean
  unstaged: boolean
  untracked: boolean
  conflicted: boolean
}
