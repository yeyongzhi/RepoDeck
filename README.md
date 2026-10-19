# RepoDeck

本地桌面 Git 仓库工作台，适合同时维护多个项目的开发者，集中查看仓库状态并完成日常 Git 操作。

当前代码版本：**0.0.1-beta1（测试版）**。

[查看本次预发布及安装包](https://github.com/yeyongzhi/RepoDeck/releases/tag/v0.0.1-beta1) · [产品文档](https://yeyongzhi.github.io/RepoDeck/) · [更新日志](docs/changelog.md)

本版本的安装包在 GitHub Release 发布完成后提供下载，属于预发布版本。

## 核心功能

- 添加本地仓库、扫描代码目录，按分组、置顶、搜索和状态筛选管理项目。
- 从远程添加或快速克隆，查看进度与日志，停止时选择保留或删除本次克隆文件。
- 查看分支、提交历史和工作区，执行 Fetch、快进拉取、提交、推送及分支操作。
- 任务中心记录克隆与 Git 操作；失效路径可重新定位，编辑器和终端支持自定义。
- SQLite 保存仓库登记、路径、分组和置顶等业务数据，支持导入导出与导入前备份；界面偏好使用 JSON。

## 安装与使用

1. 在本次预发布页面下载 Windows x64 的 `RepoDeck_0.0.1-beta1_x64-setup.exe`，运行安装程序。
2. 本机安装 Git 并配置 HTTPS 凭据或 SSH 密钥；系统需要 WebView2 Runtime。
3. 启动后点击「添加仓库」选择本地项目，或选择「从远程添加」。顶部加号提供快速克隆。

安装目录需可写，业务数据位于 `data/repodeck.sqlite`，偏好位于 `setting/preferences.json`。仓库代码仍保留在原目录。升级或卸载前建议在「设置 → 数据」导出备份。

完整说明：[安装](docs/install.md) · [使用](docs/usage.md) · [功能](docs/features.md)。本次仅提供 Windows x64 安装包，其他平台未构建验证。

## 开发简介

前端位于 `src`，Tauri / Rust 后端位于 `src-tauri`。需要 Node.js 24、pnpm 12.4.2、Rust 和 Git；Windows 构建还需要 Visual Studio C++ Build Tools 与 WebView2。

```sh
pnpm install --frozen-lockfile
pnpm tauri:dev
pnpm build
pnpm tauri build --bundles nsis
pnpm docs:dev
pnpm docs:build
```

开发版数据位于 `%LOCALAPPDATA%/RepoDeck-dev`。运行 `node --test tests/*.test.mjs` 和 `cargo test --manifest-path src-tauri/Cargo.toml --lib` 检查核心逻辑。`pnpm dev` 后的 `/tests/preview.html` 使用模拟数据，不操作真实仓库。

## 技术栈

Vue 3、TypeScript 与 Pinia 实现界面及状态管理；Tailwind CSS 和 shadcn-vue 提供组件；Vite 构建前端；Tauri 2 / Rust 调用系统 Git 并管理本地文件；rusqlite / SQLite 保存业务数据；VitePress 提供产品文档。

批量同步先 Fetch，仅在工作区干净且满足快进条件时自动拉取。Git 凭据由本机 Git 配置管理，导出的业务数据包不包含仓库代码或 Git 凭据。

[项目说明书](docs/project-guide.md) · [技术设计](docs/technical-design.md) · [产品完善清单](docs/product-improvements.md) · [开发历史](docs/development-history.md)

## 联系作者

[GitHub](https://github.com/yeyongzhi) · [反馈问题](https://github.com/yeyongzhi/RepoDeck/issues)
