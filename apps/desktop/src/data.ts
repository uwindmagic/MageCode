export const tools = [
  { id: 'chatgpt', name: 'ChatGPT', short: 'ChatGPT', color: '#8c7cdd', position: '0% 0%', role: '把想法变成清晰的下一步', initials: 'G' },
  { id: 'claude', name: 'Claude Code', short: 'Claude', color: '#d58c65', position: '33.333% 0%', role: '让复杂的工程，一步步落地', initials: 'C' },
  { id: 'trae', name: 'Trae（CN）', short: 'Trae CN', color: '#3caa95', position: '66.667% 0%', role: '专注代码，也关注每一个细节', initials: 'T' },
  { id: 'kimi', name: 'Kimi', short: 'Kimi', color: '#797bbf', position: '100% 0%', role: '从长文档里，找到关键线索', initials: 'K' },
  { id: 'deepseek', name: 'Deepseek Harness', short: 'Deepseek', color: '#628bd2', position: '0% 100%', role: '以推理串联问题和答案', initials: 'D' },
  { id: 'workbuddy', name: 'Workbuddy', short: 'Workbuddy', color: '#bd9276', position: '33.333% 100%', role: '让知识与工作井井有条', initials: 'W' },
  { id: 'doubao', name: '豆包工作', short: '豆包工作', color: '#d784a4', position: '66.667% 100%', role: '让日常工作的灵感自然发生', initials: '豆' },
  { id: 'qwen', name: '千问办工', short: '千问办工', color: '#bb9a4e', position: '100% 100%', role: '把业务问题，整理成行动方案', initials: '千' },
] as const;
export type Tool = typeof tools[number];
export type SessionStatus = 'running' | 'waiting' | 'completed' | 'idle';
export type Message = { role: 'user' | 'assistant'; text: string; time: string };
export type Session = { id: string; tool: string; toolName: string; project: string; projectName: string; nativeProject: string; title: string; purpose: string; updated: string; status?:SessionStatus; lastActiveAt?:string; archived?: boolean; source?: 'codex-local'; nativeId?: string; sourceUrl?: string; messages: Message[]; tasks: { title: string; done: boolean }[]; files: string[] };
export const projects = [
  { id: 'knowledge', name: '知识库管理网站', description: '让零散的知识，变成随手可用的资料', icon: 'folder' },
  { id: 'agent-hub', name: 'AI Agent 中台', description: '看见每个 Agent 正在做什么', icon: 'sparkles' },
  { id: 'content', name: '内容与视频工作流', description: '从一个想法，到一份完整的内容', icon: 'film' },
  { id: 'commerce', name: '电商运营与分析', description: '整理业务线索与运营行动', icon: 'chart' },
];
function session(id: string, toolId: string, projectId: string, title: string, purpose: string, nativeProject: string, tasks: string[], files: string[]): Session {
  const tool = tools.find(t => t.id === toolId)!;
  const project = projects.find(p => p.id === projectId)!;
  const exampleStatuses:SessionStatus[]=['idle','running','waiting','running','idle','completed','waiting','idle','completed'];
  const index=Number(id.slice(1))-1;
  return { id, tool: toolId, toolName: tool.name, project: projectId, projectName: project.name, nativeProject, title, purpose, updated: '10:24',status:exampleStatuses[index]||'idle',lastActiveAt:new Date(Date.now()-[44,60,60,120,120,120,1440,1440,1440][index]*60000).toISOString(),
    messages: [{role:'user',text:`这次我们来做${title}。${purpose}。`,time:'10:21'}, {role:'assistant',text:`我把这项工作整理成了下面的步骤，方便你查看进展。\n\n${purpose}。先确认范围，再逐项完成并核对结果。`,time:'10:24'}],
    tasks: tasks.map((title, i) => ({title,done:i===0})), files };
}
export const initialSessions: Session[] = [
  session('s1','chatgpt','knowledge','目录结构与索引规划','梳理知识库层级，定义文档索引与搜索范围','知识管理 / 目录索引',['确认目录边界','设计索引字段','检查检索覆盖范围'],['索引方案.md','目录清单.md']),
  session('s2','claude','knowledge','搜索体验优化','优化文档检索、结果预览与本地文件定位','Projects / 知识库网站',['梳理索引与目录结构','优化关键词匹配与结果预览','验证资源管理器定位'],['search.ts','explorer.ts']),
  session('s3','workbuddy','knowledge','知识内容整理','按主题清理与结构化现有文档内容','知识工作 / 资料整理',['整理资料来源','补齐主题标签','核对重复内容'],['知识库目录.md']),
  session('s4','trae','agent-hub','基础工作台界面','实现四栏布局、工具切换和会话导航','Workspace / AI_MP_AgentTasks',['确认界面布局','实现导航交互','检查小屏布局'],['App.tsx','styles.css']),
  session('s5','deepseek','agent-hub','多工具会话归类','梳理统一项目与原生项目之间的映射规则','Harness / 中台研究',['比较来源结构','设计项目映射','检查同名冲突'],['会话映射.md']),
  session('s6','kimi','content','长文素材与选题整理','从长文档中提取选题与可引用的材料','创作 / 选题资料',['阅读素材目录','提取选题线索','记录引用出处'],['选题清单.md']),
  session('s7','doubao','content','口播脚本与分镜','把选题拆成可录制的口播与分镜','我的工作 / 视频创作',['明确表达目标','整理口播结构','拆解分镜画面'],['口播脚本.md','分镜表.md']),
  session('s8','qwen','commerce','商品分析与运营计划','整理商品卖点、竞品信息与运营动作','业务 / 电商运营',['收集商品信息','比较卖点差异','整理运营行动'],['商品分析.md']),
  {...session('s9','chatgpt','agent-hub','早期需求讨论','保留已完成的早期需求讨论，便于回溯','知识管理 / 中台规划',['归档需求'],['需求草稿.md']),archived:true},
];
export const storageKey = 'agent-tasks-v0.1';
export function loadSessions(): Session[] {
  try {
    const value = JSON.parse(localStorage.getItem(storageKey) || 'null');
    if (Array.isArray(value) && value.length && value.every(s => s && typeof s.id === 'string' && tools.some(t=>t.id===s.tool) && projects.some(p=>p.id===s.project) && typeof s.title==='string' && typeof s.purpose==='string' && Array.isArray(s.messages) && s.messages.every((m: Message)=>m&&['user','assistant'].includes(m.role) && typeof m.text==='string') && Array.isArray(s.tasks) && s.tasks.every((t:{title:string;done:boolean})=>t&&typeof t.title==='string'&&typeof t.done==='boolean') && Array.isArray(s.files)&&s.files.every((f:unknown)=>typeof f==='string'))) {
      return value.map(s=>{const seed=initialSessions.find(i=>i.id===s.id);return {...s,status:['running','waiting','completed','idle'].includes(s.status)?s.status:seed?.status||'idle',lastActiveAt:s.lastActiveAt||seed?.lastActiveAt};});
    }
  } catch { /* Invalid demo state falls back to bundled examples. */ }
  return initialSessions;
}
