# 统一数据库与双视图设计 · v0.3

## 原则

MageCode DB 是所有工具共用的本地展示投影，不是 Agent 原始历史备份。默认位置为 Windows `%APPDATA%\MageCode\magecode.db`；测试可用 `MAGECODE_DATA_DIR` 隔离。启用 foreign keys、WAL 与 5 秒 busy timeout。

## 表与职责

| 表 | 关键身份 / 职责 |
|---|---|
| `tools` | 工具类型，不等同模型或连接 |
| `connections` | 本机来源实例、来源根目录、只读标记、最后同步时间 |
| `hub_projects` | 规范化共享主目录身份；跨工具“按项目”聚合点 |
| `native_projects` | `(connection_id,native_project_id)` 唯一；保留原生路径、父级和显示名 |
| `project_mappings` | native → hub；精确规范化目录自动映射，保留映射依据 |
| `sessions` | `(connection_id,native_session_id)` 唯一；原生项目、来源定位、标题、归档、原生时间 |
| `session_project_links` | 会话级项目覆盖；优先于项目默认映射的扩展位 |
| `messages` | 会话 + 原生消息 ID 去重；仅 user/final assistant 正文、顺序和源时间 |
| `session_annotations` | 用户用途和注释独立保存，重同步不覆盖 |
| `sync_runs` | 手动运行状态、起止时间、会话/消息数量、错误扩展位 |
| `sync_cursors` | 来源版本与检查点扩展位；v0.3 仅记录本次快照计数，不宣称增量同步 |

## 展示过滤

Codex 在写入前过滤，而非仅前端隐藏：developer、工具调用/结果、assistant commentary、缺失或未知 phase 的 assistant 均不写入 `messages`。因此后续其他 UI 也不会误读过程正文。原始来源保持不变，可在原工具查看完整记录。

## 双视图

- 按项目：`hub_projects → project_mappings → native_projects → sessions`。只有相同规范化主目录自动合并；同名、父子目录不合并。
- 按工具：`tools → connections → native_projects → sessions`，使用原生路径，不经过 hub 合并。
- 两种视图读取同一 `sessions` 行，不复制会话。

当前仅 Codex 有真实连接，所以跨工具自动归并规则已由模型和路径测试建立，但尚无第二个真实连接器可展示跨工具样例。

## 同步与保留

启动只打开已有快照；设置页“同步 Codex”触发一次扫描、过滤和事务提交，无定时任务。单会话消息按原生 ID 重建展示投影，会话和消息幂等。未在本次来源出现的缓存会话暂不删除，避免来源暂时不可读造成数据丢失；删除语义和增量同步规则留待后续决定。
