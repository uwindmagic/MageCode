# MageCode · v0.3.0

浅色/深色四栏工作台：以共享主目录或工具原生项目组织会话。本版建立所有工具共用的本地 SQLite 展示库，并接入 Codex 手动同步；其他 Agent 仅列入已核验的来源调查矩阵。

## 启动

需要 Node.js 22 LTS 和 npm。在本目录执行：

```sh
npm install
npm run dev
```

浏览器入口 `http://127.0.0.1:1420`。Windows PowerShell 若拦截 npm.ps1，使用 `npm.cmd` 执行同样命令，无需修改系统执行策略。

桌面窗口：安装 Rust stable、平台 Tauri 编译前置环境后执行 `npm run desktop`。Windows 需要 MSVC C++ Build Tools 与 WebView2。`npm run desktop:build` 用于编译桌面二进制；v0.1 不生成安装包。

```sh
npm test
npm run build
```

## 本版范围

- 8 个工具：ChatGPT、Claude Code、Trae（CN）、Kimi、Deepseek Harness、Workbuddy、豆包工作、千问办工。
- ChatGPT 栏目的“Codex 本地 · 只读”来源已接入统一库：设置页手动同步默认 `.codex`，列表和正文从 SQLite 读取；启动仅读取已有快照。
- 展示库只保存用户消息和 `phase=final` 的 Codex 最终答复；commentary、缺失 phase 的旧助手记录、工具调用和分析过程不会入库或显示。
- “按项目”使用规范化共享主目录身份，可映射不同工具的原生项目；“按工具”保留原生目录。两种视图引用同一会话。当前只有 Codex 真实连接，跨工具真实归并待后续连接器验收。
- 真实会话支持标题和项目搜索、原 Codex 会话跳转；已加载正文可参与当前前端搜索。全库正文索引搜索尚未实现。
- 本地演示会话创建、用途编辑、消息、归档与恢复；刷新保留演示数据。Hash URL 支持前进、后退、直接访问。
- Agent 圆形头像、右侧角色面板、演示状态、最近活动时间和浅色/深色主题。

真实 Codex 记录与演示数据分开管理：真实正文不会写入 localStorage，界面不提供真实会话的消息发送、状态修改、任务勾选或归档操作。实时运行状态、精确 Token、双向续聊、网页 ChatGPT 全账号历史同步仍未接入。Codex 原会话跳转通过系统协议处理器请求打开，是否成功交由本机 Codex 客户端决定。

统一库默认位于 `%APPDATA%\MageCode\magecode.db`，不放在网盘目录。当前未创建定时同步任务，增量规则以后确定；通用 core/infrastructure 分层、其他 Agent 连接器、模型调用和系统文件定位尚未实现。浏览器预览仅显示演示数据；真实本地读取仅在 Tauri 桌面窗口启用。演示数据保存在当前浏览器/WebView 的 localStorage，二者不自动共享。

Rust 工具链由 rust-toolchain.toml 固定为 1.90.0，不改变系统默认。桌面依赖由 Cargo.lock 锁定。

## 架构与记录

- [系统架构](ARCHITECTURE.md)
- [接口设计](docs/architecture/contracts.md)
- [架构决策](docs/adr/decisions-v0.1.md) · [ADR-002](docs/adr/ADR-002_统一展示投影与共享目录身份.md)
- [统一数据库设计](docs/architecture/unified-database-v0.3.md) · [Agent 本地结构调查](docs/architecture/local-source-investigation-v0.3.md)
- [版本索引](docs/versions/README.md)
- [需求对话记录入口](docs/requirements/README.md)

历史版本记录、需求原文、设计素材与验收证据仍保存在 `H:\PARA\2.Area\AI中台_Agent都在干什么_20261026`；从 v0.2.0 起，仓库内 `docs/versions` 与 `docs/requirements` 同步保留正式记录，每版单独建文件，不覆盖旧版。

角色图由 imagegen 生成，用于界面头像/立绘，不代表各工具官方形象。项目保留既有 `middle__platform.md.txt`。

新版 Windows 调试程序为 `target/debug/MageCode.exe`。
