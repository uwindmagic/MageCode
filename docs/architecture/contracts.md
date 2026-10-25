# ApplicationClient 与 Rust 接口契约 v0.2.0

状态：目标契约 + v0.3.0 本地数据库实现。Tauri IPC 为首期传输；未来 HTTP 映射不改变领域含义。下列通用规则仍是目标；当前四个临时命令尚未引入分页、schema_version、结构化错误和自动生成类型，DTO 暂时使用 camelCase 以匹配现有前端，提取正式 core 时必须对齐目标契约。

## v0.3.0 已实现的临时命令

| 命令 | 输入 | 返回 / 边界 |
|---|---|---|
| list_codex_sessions | 无 | 从统一 SQLite 返回来源根目录、最后同步标识和会话摘要；不扫描原始历史，不返回正文 |
| sync_codex | 无 | 手动扫描默认 `.codex`，仅将用户消息与 `phase=final` 助手答复写入统一库，事务完成后返回快照 |
| read_codex_messages | native_id | ID 格式校验后，从统一库按顺序读取指定会话的可见消息；不回读原始分析或工具过程 |
| open_codex_session | native_id | 校验原会话存在后，用系统协议处理器打开对应 `codex://threads/<id>`；不拼接 shell 命令 |

这些命令是当前过渡接口，不是完整 ApplicationClient 契约。浏览器没有文件系统权限，因此只运行演示模式。

## 通用规则

- ID 为不透明字符串，不用显示标题或绝对路径作主键。
- 时间为 UTC RFC3339；界面按用户时区显示。
- DTO 显式带 schema_version；字段名 snake_case；枚举显式序列化。
- 可选用量值用 null 表示未知，0 表示已知为零；不使用 NaN/Infinity。
- 所有列表分页并带稳定排序/游标；游标失效返回结构化错误，客户端重读。
- Rust DTO 生成 TypeScript，构建检查生成结果一致性。输入在 Rust 运行时校验，类型生成不代替校验。
- 可恢复错误返回 code、message、retryable、correlation_id；details 不包含凭据与原始敏感路径。

## 查询和本地管理命令

| 命令 | 主要输入 | 返回 / 行为 |
|---|---|---|
| list_tools | 无 | 工具定义、连接实例、逐项能力及更新时间 |
| list_project_tree | view=hub/tool、connection_id?、parent_id? | 当前层节点、工具标识、子节点状态；懒加载 |
| list_sessions | hub_project_id?/native_project_id?/tool_id?、cursor?、limit | 会话摘要、用途来源、工具/连接、next_cursor |
| get_session | session_id | 来源身份、映射、标题、用途、任务和数据新鲜度 |
| get_messages | session_id、cursor?、limit | 有序消息、来源消息 ID、next_cursor |
| search | query、filters、cursor?、limit | 命中摘要、session_id、message_id?、来源工具、覆盖范围 |
| save_annotation | session_id、expected_revision、patch | 新 revision；冲突返回 CONFLICT |
| map_project | hub_project_id、native_project_id?/session_id?、expected_revision | 保存用户确认的映射；来源数据保持不变 |
| start_sync | connection_id、mode=incremental/full | job_id；full 重建缓存时保留用户数据 |
| cancel_sync | job_id | 请求取消状态，允许任务尚未停止 |
| reveal_source | session_id、source_ref_id | 平台文件定位结果；只允许后端已知来源引用 |
| open_native_session | session_id | opened / unsupported / unavailable；校验适配器能力 |

来源读取涉及凭据与路径配置的连接管理命令在 P0 来源探测后细化，不能通过前端传任意 shell 或 SQL 来代替。

## 事件

统一封套：schema_version、event_id、revision、occurred_at、connection_id?、payload。

| 事件 | payload 要点 |
|---|---|
| sync_progress | job_id、phase、processed、total?、errors、状态 |
| index_changed | 受影响实体 ID、提交后的数据 revision |
| connection_changed | 状态、逐项能力、原因、检测时间 |
| usage_updated | session_id、指标 scope 与观察时间 |
| run_event（后期） | run_id、sequence、类型、增量内容/审批引用 |

事件仅用于通知，数据库快照为查询依据。应用切回前台、检测 revision 跳跃或事件丢失时重新拉取；后期运行事件断线恢复需按 sequence 补拉或查询原生状态。

## 状态分离

- connection：unknown / ready / needs_auth / unavailable / unsupported。
- sync：idle / scanning / indexing / failed / cancelled / completed。
- run（后期）：queued / running / awaiting_approval / cancelling / cancelled / completed / failed / unknown。
- freshness：fresh / stale / source_missing，附 last_success_at。

sync completed 不推导 run completed；取消请求成功不等于原生执行已停止。

## Agent 端口的职责

core 定义接口，infrastructure 实现。最小读取接口覆盖 probe、分页 list_projects、list_sessions、read_messages 和 read_usage；每个调用接受取消信号并返回来源版本/检查点。用 Result 返回已分类错误。

后续执行接口独立于读取接口：resume(session_identity)、send(run_id, request_id, message)、cancel(run_id)、respond_approval(approval_id, decision)。不支持能力返回 UNSUPPORTED，不生成虚假空成功结果。

适配器返回原生身份、消息和来源状态；中台仓储负责事务与幂等去重，适配器不直接操作中台业务表。平台文件与进程操作走受限系统服务。

## 后期会话交互必须满足

发送请求携带 request_id，关联原会话与连接。断线/超时后先查运行状态，不能不加判断再次发送。只有工具真正提供幂等能力时才能保证原生端严格幂等；其余情况标记结果未知并要求明确后续操作。

审批显示原生动作和范围，由用户处理；中台不能因为历史消息中的文字而自动批准。新增 HTTP 入口时补认证、授权、CSRF/来源校验及事件订阅范围。

## 错误码设计

NOT_FOUND、INVALID_ARGUMENT、CONFLICT、UNSUPPORTED、NEEDS_AUTH、SOURCE_UNAVAILABLE、SOURCE_FORMAT_UNSUPPORTED、CANCELLED、TIMEOUT、INTERNAL。

IO 错误区分来源缺失、权限不足、锁定、损坏。客户端可显示脱敏原因及最后可用时间；详细诊断通过 correlation_id 查询本地日志。
