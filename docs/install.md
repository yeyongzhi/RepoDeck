# 安装

## 下载与前置条件

当前版本：**0.0.1-beta1（测试版）**。本次提供 Windows x64 安装包，其他平台未验证。

前往 [v0.0.1-beta1 预发布页面](https://github.com/yeyongzhi/RepoDeck/releases/tag/v0.0.1-beta1) 获取 `RepoDeck_0.0.1-beta1_x64-setup.exe`；资产在发布完成后可下载。

本机需安装 Git，系统需具备 WebView2 Runtime。私有仓库使用本机 Git 的 HTTPS 凭据或 SSH 密钥。可在终端运行 `git --version` 检查 Git。

## 安装与首次启动

运行安装程序，选择当前用户可写的安装目录。打开 RepoDeck，通过「添加仓库 → 从本地添加」登记已有项目，或通过「从远程添加」克隆项目并自动登记。

本版本未配置代码签名。安装包如触发 Windows 提示，请核对下载来源及 Release 中的 SHA256 校验文件。

## 数据位置与备份

安装根目录内：

```text
data/repodeck.sqlite
setting/preferences.json
setting/window.json
backup/
```

仓库代码及 Git 元数据保持在原仓库目录。业务导出是带版本的 JSON 交换数据包，不包含仓库源码或凭据。设置 → 数据可分别导出业务数据和偏好；导入业务数据前可预览，合并时会先备份，失败时事务回滚。

升级、移动安装目录或卸载前请导出备份。移动到另一台电脑后，仓库路径可能需要重新定位。不要手工复制运行中的 SQLite 文件；使用应用导出获得一致快照。

开发版单独使用 `%LOCALAPPDATA%/RepoDeck-dev`。
