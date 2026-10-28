# MageCode · v0.2

跨 Agent 项目/会话工作台。支持浅色与深色主题；ChatGPT 栏目新增只读资料接入，其他工具仍使用演示数据。

## 运行

Windows 桌面程序：`target/debug/MageCode.exe`，已打包界面资源，不依赖 Vite 提供页面。点击左侧导航标题行的 **ChatGPT 资料接入**（下载图标），选择 **读取默认目录**。

默认目录 `~/.codex` 的来源明确标为 Codex 本地，不能当作网页 ChatGPT 全账号历史。可另行粘贴聊天摘录或导入 TXT/Markdown，保存原会话链接。

浏览器开发预览需运行两个本机进程：

```sh
target/debug/MageCode.exe --source-server
npm run dev
```

来源服务只监听 127.0.0.1:1421，Vite 在 127.0.0.1:1420 同源代理。数据留在本机；不启用自动启动或外网服务。桌面正常运行经 Tauri IPC 读取，不额外开启来源端口。

## 开发检查

```sh
npm install
npm test
npm run build
cargo test -p magecode-local-source
cargo run -p magecode-local-source --example probe
npm run desktop:build -- --debug
```

Node 22 LTS；项目 Rust 工具链固定 1.90.0；Windows 需 MSVC C++ Build Tools/WebView2。PowerShell 可用 npm.cmd 避开 npm.ps1 执行策略限制。

## 数据边界

- 只读本机 state_5.sqlite 的项目/会话及 thread_history_1.sqlite 的用户/助手可见文本，排除内部审查/子代理、私有推理和工具输出。
- 来源数据、读取时间、用户用途/归类、用户标记分开。导入成功不等于正在运行；运行状态无可靠事件则未知。
- 只显示来源实际提供的模型；Token、上下文、额度无可靠计量则未知。附件和分支尚未接入。
- 原会话打开由用户点击请求系统处理，尚未完成目标聊天核验；不提供原会话消息发送或续聊。
- 来源正文不写入 localStorage；元数据与备注保存本机，刷新时按需读取。手工文本摘录按用户意图保存在当前浏览器/WebView。
- 再次读取按来源 ID 合并，保留用户归类/用途；快照不含某条记录时不删除本地历史。

## 当前验证

前端生产构建、Windows 二进制、5 项 Node 检查和 1 项 Rust 检查通过。实读接口获得 139 个会话并排除 249 条内部记录（本次观察值）；样本可能缺失助手回复，界面已实现覆盖提示。

完整 UI 导入流程和原会话目标跳转尚未验收：浏览器控制停在错误页，独立桌面应用控制授权超时。不能将接口检查当作 UI 端到端成功。

## 文档

- [系统架构](ARCHITECTURE.md)
- [接口设计](docs/architecture/contracts.md)
- [版本索引](docs/versions/README.md)
- [需求原文索引](docs/requirements/README.md)

完整版本文档、用户需求原文和证据位于 `H:\PARA\2.Area\AI中台_Agent都在干什么_20261026`。本轮遵循 ponytail，只增加已使用的 Rust 来源模块和预览桥接，不预建其他 Agent 适配器。
