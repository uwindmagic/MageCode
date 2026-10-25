# Agent 本地数据结构调查 · v0.3

日期：2026-10-25
方法：SQLite 使用只读 URI 与 `backup()` 生成一致性副本后检查；JSONL 仅提取字段、类型和内容块类型。本记录不包含聊天正文、凭据或完整私有路径。

## 支持矩阵

| 工具 | 已验证的本机结构 | 可见结果判定 | v0.3 状态 |
|---|---|---|---|
| Codex | `.codex/sessions`、`archived_sessions` JSONL；`session_index.jsonl`；`session_meta` / `response_item` | 用户 message + `phase=final` 的助手 message | 已接入手动同步、统一库、双视图、原会话跳转 |
| Kimi | `conversations.sqlite` 的 `sessions` / `conversations`；对话含工作区、标题、时间与 `kernel_records_path`；对应 JSONL 含 `turn.prompt`、`context.append_message`、`context.append_loop_event`、LLM 与工具事件 | 候选正文事件已定位，但需逐事件确认最终答复与分支语义；loop/tool 事件明确不展示 | 已定位，未接入 |
| 千问办公 | `projects/<编码目录>/<session>.jsonl`；`workspace-directories` 提供真实工作区；`user` / `assistant` 记录带 `cwd`、`uuid`、`parentUuid`、时间 | 内容块已验证有 `text`、`thinking`、`tool_use`、`tool_result`；后三类不得作为最终答复直接导入 | 已定位，未接入 |
| WorkBuddy | `workbuddy.db` 的 `sessions`（cwd、标题、项目、时间、删除状态）与 `workspaces`；另有 session 工作目录 | 已抽查会话目录只发现修改备份等文件，尚未验证正文主存储 | 元数据已定位，正文未定位，未接入 |
| Trae CN | `state.vscdb` 为 `ItemTable(key,value)`；抽查 205 个键，含会话关系、建议缓存与 UI 状态 | 未确认主聊天正文键或独立聊天库；建议缓存不当作对话结果 | 未确认正文，未接入 |
| Claude Code | `.claude/sessions` 当前为空 | 无真实样本 | 未支持 |
| 豆包工作 | `DoubaoWork/chats` 未发现可用聊天样本 | 无真实样本 | 未支持 |
| Deepseek Harness | 已见设置、插件和 Chromium 缓存，未定位会话主库 | 不把缓存数据库当聊天记录 | 未支持 |

## 对统一模型的约束

- Kimi 的 `conversation_key`、`conversation_id`、kernel 记录定位与工作区应分别保留，不能只用标题去重。
- 千问项目身份以记录内 `cwd` / `workspace-directories` 为准，不通过目录名反转猜测路径。
- WorkBuddy 原生 `project_id`、`cwd` 和软删除状态可进入元数据适配层，但正文读取验证前不开放展示。
- Trae 必须区分 TraeCode / TraeWork 及全局缓存、会话正文；未验证前不写适配器。
- 所有来源只读；自有迁移只作用 MageCode DB。后续同步应先保存来源版本和游标，再提交展示投影。
