import type { RepositorySummary } from '@/types/git'
export function actionReason(repo: RepositorySummary, action: 'pull' | 'push' | 'checkout' | 'commit'): string {
 if (repo.hasConflicts) return '先在编辑器中解决冲突，再刷新仓库'
 if (repo.operationInProgress) return '先完成当前 Git 操作，再刷新仓库'
 if (action === 'commit') return repo.isDirty ? '' : '没有需要提交的改动'
 if (repo.isDirty) return '先提交或 Stash 保存改动'
 if (action === 'checkout') return ''
 if (repo.isDetached) return '先切换到本地分支'
 if (!repo.upstream) return '先在分支设置中关联远程分支'
 if (action === 'pull') return repo.ahead > 0 ? '本地有独有提交，请先对比分支' : repo.behind === 0 ? '本地引用没有待拉取提交，可先 Fetch 检查' : ''
 return repo.behind > 0 ? '先获取并整合远端提交' : repo.ahead === 0 ? '当前没有待推送提交' : ''
}
