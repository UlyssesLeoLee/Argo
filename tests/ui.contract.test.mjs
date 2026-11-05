import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {PROVIDERS,validateTask,makeTask,normalizeCatalog,escapeHtml,stateLabels} from '../web/lib.js';
const base=()=>makeTask({provider:'codex',title:'Fix API',prompt:'Use tests before edits',context:{}});
test('UT-UI-01 seven unique provider cards',()=>{
 assert.equal(PROVIDERS.length,7);
 assert.equal(new Set(PROVIDERS.map(p=>p.id)).size,7);
});
test('UT-UI-02 standard task valid',()=>assert.equal(validateTask(base()),null));
test('UT-UI-03 rejects missing input',()=>assert.match(validateTask({...base(),prompt:' '}),/任务目标/));
test('UT-UI-04 requires Orca repository and agent',()=>{
 const t={...base(),provider:'orca',context:{orca_repo_id:'r'}};
 assert.match(validateTask(t),/Orca/);
 assert.equal(validateTask({...t,context:{orca_repo_id:'r',orca_agent:'codex'}}),null);
});
test('UT-UI-05 requires Multica Workspace / Project / Agent',()=>{
 const t={...base(),provider:'multica',context:{multica_workspace_id:'ws'}};
 assert.match(validateTask(t),/Agent/);
 assert.equal(validateTask({...t,context:{multica_workspace_id:'ws',multica_project_id:'p',multica_assignee:'Builder'}}),null);
 assert.equal(validateTask({...t,context:{multica_workspace_id:'ws',multica_issue_id:'MUL-123',multica_assignee:'Builder'}}),null);
 assert.match(validateTask({...t,context:{multica_mode:'existing',multica_workspace_id:'ws',multica_project_id:'p',multica_assignee:'Builder'}}),/Issue ID/);
});
test('UT-UI-06 preserves user prompt as literal data',()=>{
 const t=makeTask({provider:'claude',title:'hi',prompt:'hello\n$(dangerous) && :;'});
 assert.equal(t.prompt,'hello\n$(dangerous) && :;');
});
test('UT-UI-07 escapes external strings',()=>assert.equal(escapeHtml('<img src=x onerror="1">'),'&lt;img src=x onerror=&quot;1&quot;&gt;'));
test('UT-UI-08 parse project, issue, repo lists',()=>{
 assert.deepEqual(normalizeCatalog('{"projects":[{"id":"p1","title":"Sprint"}]}','project'),[{id:'p1',label:'Sprint'}]);
 assert.equal(normalizeCatalog('[{"id":"42","name":"Star"}]','repo')[0].id,'42');
 assert.equal(normalizeCatalog('not json','issue').length,0);
});
test('UT-UI-09 state machine labels cover dispatched and needs review',()=>{
 assert.equal(stateLabels.dispatched,'已派发');assert.ok(stateLabels.needs_review);
});
test('UT-UI-10 UI references correct routes',()=>{
 const text=readFileSync(new URL('../src/app.ts',import.meta.url),'utf8');
 for(const path of ['/api/v1/tasks','/api/v1/nodes','/api/v1/providers']) assert.ok(text.includes(path), `UI must use ${path}`);
 const server=readFileSync(new URL('../crates/argo/src/server.rs',import.meta.url),'utf8');
 assert.ok(server.includes('/api/v1/blobs'), 'blob transfer is a server/worker protocol, not a direct UI call');
});
