export function explainGitError(raw: string): {title: string; action: string} {
  const rules: [RegExp,string,string][] = [
    [/Authentication failed|Permission denied \(publickey\)|could not read Username|认证/i,'远程认证失败','检查本机 Git 凭据或 SSH 密钥，确认当前账号有仓库访问权限。'],
    [/Host key verification failed/i,'SSH 主机尚未受信任','在终端核实并建立该主机的 SSH 信任后重试。'],
    [/Could not resolve|Failed to connect|timed out|connection reset|无法连接|网络/i,'无法连接远程仓库','检查网络、代理和远程地址后重试。'],
    [/already exists|已存在|not an empty directory/i,'目标目录已存在','更换项目目录名，或打开该目录，完整仓库可从本地添加。'],
    [/Permission denied|Access is denied|拒绝访问|不可写|os error 5/i,'目录没有写入权限','选择当前用户可写的目录，并检查文件是否被其他程序占用。'],
    [/not found|does not exist|无法访问|不存在/i,'路径或远程仓库不可访问','确认目录、仓库地址和账号权限；仓库移动后可重新定位。'],
    [/conflict|冲突|unmerged/i,'存在需要处理的冲突','在编辑器或终端解决冲突后刷新仓库，再继续操作。'],
    [/non-fast-forward|diverged|分叉/i,'本地与远端存在分叉','先获取远端更新并对比分支，再选择合并方式。'],
  ]
  for (const [pattern,title,action] of rules) if (pattern.test(raw)) return {title,action}
  return {title:'操作未完成',action:'查看详细日志，确认仓库状态后重试。'}
}
