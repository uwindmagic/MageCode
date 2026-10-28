import test from 'node:test';
import assert from 'node:assert/strict';
import { parseRoute, routeFor, filterSessions } from '../apps/desktop/src/navigation.mjs';
import {agentStatus,relativeTime,sessionStatus} from '../apps/desktop/src/status.mjs';
test('demo status aggregation ignores archives; relative activity handles minute/hour/day and unknown',()=>{
  const now=Date.parse('2026-10-25T15:00:00Z');
  assert.deepEqual([44,60,1440].map(m=>relativeTime(new Date(now-m*60000).toISOString(),now)),['44 分','1 时','1 天']);
  assert.equal(relativeTime(undefined,now),'未知');
  assert.equal(sessionStatus({archived:true,status:'running'}),'archived');
  assert.equal(agentStatus([{tool:'a',status:'running',archived:true},{tool:'a',status:'waiting'}],'a'),'waiting');
  assert.equal(agentStatus([{tool:'a',status:'completed'}],'a'),'idle');
});
test('routes survive refresh and tab navigation; unknown pages/tabs have safe defaults',()=>{
  const route={page:'tools',tool:'claude',session:'s2',tab:'files'};
  assert.deepEqual(parseRoute(routeFor(route)),route);
  const imported={...route,session:'codex-local:01a174b3-1a61-7830-89dd-68847d814340'};
  assert.deepEqual(parseRoute(routeFor(imported)),imported);
  assert.doesNotThrow(()=>parseRoute('#/tools/all/%bad/chat'));
  assert.equal(parseRoute('#/nonsense/all/s2/wrong').page,'workspace');
  assert.equal(parseRoute('#/nonsense/all/s2/wrong').tab,'chat');
});
test('search combines selected tool, archive state and Chinese body text without duplicate view data',()=>{
  const base={title:'检索',purpose:'整理',projectName:'知识库',toolName:'Claude Code',messages:[{text:'定位本地文件'}]};
  const sessions=[{...base,id:'a',tool:'claude'},{...base,id:'b',tool:'chatgpt'},{...base,id:'c',tool:'claude',archived:true}];
  assert.deepEqual(filterSessions(sessions,{tool:'claude',query:'本地文件'}).map(s=>s.id),['a']);
  assert.deepEqual(filterSessions(sessions,{archived:true,query:' 知识库 '}).map(s=>s.id),['c']);
  assert.equal(filterSessions(sessions,{query:'无匹配内容'}).length,0);
});
