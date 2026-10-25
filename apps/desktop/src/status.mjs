export const statusLabels = {running:'进行中',waiting:'等待处理',completed:'已完成',idle:'空闲',archived:'已归档'};
/** @returns {'running'|'waiting'|'completed'|'idle'|'archived'} */
export function sessionStatus(session) { return session.archived ? 'archived' : Object.hasOwn(statusLabels,session.status) && session.status!=='archived' ? session.status : 'idle'; }
export function relativeTime(value, now=Date.now()) {
  const time=Date.parse(value);
  if(!Number.isFinite(time))return '未知';
  const minutes=Math.max(0,Math.floor((now-time)/60000));
  return minutes<1?'刚刚':minutes<60?`${minutes} 分`:minutes<1440?`${Math.floor(minutes/60)} 时`:`${Math.floor(minutes/1440)} 天`;
}
export function agentStatus(sessions,tool) {
  const active=sessions.filter(s=>s.tool===tool&&!s.archived);
  return active.some(s=>sessionStatus(s)==='running')?'running':active.some(s=>sessionStatus(s)==='waiting')?'waiting':'idle';
}
