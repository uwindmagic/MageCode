import test from 'node:test';
import assert from 'node:assert/strict';
import {mergeImported,safeChatLink} from '../apps/desktop/src/source-client.ts';
test('reimport is idempotent and keeps local purpose, project, status and archive',()=>{
  const source={id:'codex-local:test',title:'来源标题',purpose:'未整理',project:'chatgpt-inbox',projectName:'待归类',messages:[],tasks:[],status:'unknown',source:{kind:'codex_local',sourceTitle:'来源标题',observedAt:1}};
  const local={...source,purpose:'用户用途',project:'knowledge',projectName:'知识库',status:'completed',archived:true};
  const merged=mergeImported([local],[{...source,source:{...source.source,observedAt:2}}]);
  assert.equal(merged.length,1);assert.equal(merged[0].purpose,'用户用途');assert.equal(merged[0].project,'knowledge');assert.equal(merged[0].status,'completed');assert.equal(merged[0].archived,true);assert.equal(merged[0].source.observedAt,2);
  assert.equal(mergeImported(merged,[source]).length,1);
});
test('original links permit only ChatGPT conversation HTTPS addresses',()=>{
  assert.equal(safeChatLink('https://chatgpt.com/c/sample-id'),'https://chatgpt.com/c/sample-id');
  for(const url of ['javascript:alert(1)','https://chatgpt.com.evil/c/x','https://user:pass@chatgpt.com/c/x','https://chatgpt.com/share/x','file:///etc/passwd'])assert.throws(()=>safeChatLink(url));
});
