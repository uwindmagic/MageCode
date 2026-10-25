export const tabs = ['chat', 'tasks', 'files', 'activity'];
function decodeRoutePart(value) {
  try { return decodeURIComponent(value); } catch { return value; }
}
export function parseRoute(hash) {
  const [page = 'workspace', tool = 'all', session = 's2', tab = 'chat'] = hash.replace(/^#\/?/, '').split('/');
  return { page: ['workspace', 'tools', 'archive', 'settings'].includes(page) ? page : 'workspace', tool:decodeRoutePart(tool), session:decodeRoutePart(session), tab: tabs.includes(tab) ? tab : 'chat' };
}
export function routeFor(route) {
  return `#/${route.page}/${encodeURIComponent(route.tool)}/${encodeURIComponent(route.session)}/${route.tab}`;
}
export function filterSessions(sessions, { tool = 'all', query = '', archived = false }) {
  const term = query.trim().toLocaleLowerCase();
  return sessions.filter(s => Boolean(s.archived) === archived && (tool === 'all' || s.tool === tool) &&
    (!term || [s.title, s.purpose, s.projectName, s.toolName, ...s.messages.map(m => m.text)].join(' ').toLocaleLowerCase().includes(term)));
}
