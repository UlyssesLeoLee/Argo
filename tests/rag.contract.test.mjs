import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {splitContext,cleanTags,redactPreview,buildCitationContext,validScope,canRunOnMachine,statuses} from '../web/rag.lib.mjs';
const source=readFileSync(new URL('../web/app.mjs', import.meta.url),'utf8');
const html=readFileSync(new URL('../web/index.html', import.meta.url),'utf8');
const rust=readFileSync(new URL('../crates/argo/src/rag.rs', import.meta.url),'utf8');
const route=readFileSync(new URL('../examples/vercel-rag-gateway/app/api/argo/rag/upsert/route.ts',import.meta.url),'utf8');
test('RAG-UT-01 segmentation handles CJK and emoji',()=>{
 const original='架构😀设计。'.repeat(450);const out=splitContext(original);
 assert.ok(out.length>=3);assert.ok(out.every(s=>Array.from(s).length<=850));
 assert.ok(out.some(s=>s.includes('😀')));
});
test('RAG-UT-02 preserves small paragraph',()=>assert.deepEqual(splitContext('接口设计。'),['接口设计。']));
test('RAG-UT-03 empty input safe',()=>assert.deepEqual(splitContext('  '),[]));
test('RAG-UT-04 invalid overlap disabled',()=>assert.deepEqual(splitContext('hello',100,100),[]));
test('RAG-UT-05 segment bounded',()=>assert.ok(splitContext('X'.repeat(80000)).length<=128));
test('RAG-UT-06 redacts credentials',()=>{
 const s=redactPreview('Authorization: Bearer abcdefghijk123456\napi_key=sk-123456789abcdef012');
 assert.ok(!s.includes('abcdefghijk123456'));assert.ok(s.includes('[REDACTED]'));
});
test('RAG-UT-07 tags sanitized/deduplicated',()=>assert.deepEqual(cleanTags('设计, 设计, RAG, ,系统'),['设计','RAG','系统']));
test('RAG-UT-08 rejects dangerous relative paths',()=>{
 for(const p of ['../secret','src/../secret','/etc/passwd','C:\\Users','src\\private','./src'])assert.equal(validScope(p),false,p);
 assert.equal(validScope('src/ui/'),true);
});
test('RAG-UT-09 file owner independent from runner and prevents miswrite',()=>{
 assert.equal(canRunOnMachine('A','B','Star'),false);
 assert.equal(canRunOnMachine('A','A','Star'),true);
 assert.equal(canRunOnMachine('A','B',''),true);
});
test('RAG-UT-10 citation preserves explicit source ids',()=>{
 const s=buildCitationContext([{id:'rag-a',title:'T',content:'Data'},{id:'rag-b',title:'X',content:'Context'}]);
 assert.ok(s.includes('[Argo-RAG:rag-a]'));assert.ok(s.includes('[Argo-RAG:rag-b]'));
});
test('RAG-UT-11 statuses finite',()=>assert.deepEqual(statuses,['draft','approved','synced','archived']));
test('RAG-UI-12 separate RAG page and platform tabs',()=>{
 for(const key of ['screen-rag','agentTabs','ragProviderFilter','ragProjectFilter','ragDetailId','fileMachine','scopePath','executionMachine'])assert.ok(html.includes(`id="${key}"`),key);
});
test('RAG-UI-13 remote credentials not embedded in browser',()=>{
 assert.ok(source.includes('/api/v1/rag/remote-search'));
 assert.ok(!source.includes('ARGO_RAG_API_TOKEN'));
});
test('RAG-API-14 all review endpoints exposed',()=>{
 for(const k of ['/api/v1/rag/chunks','/api/v1/rag/sync','/api/v1/rag/retrieve','/api/v1/rag/remote-search'])assert.ok(rust.includes(k),k);
});
test('RAG-API-15 remote Gateway uses trusted server-side owner and param SQL',()=>{
 assert.ok(route.includes('ARGO_RAG_OWNER_ID'));
 assert.ok(route.includes('ON CONFLICT(owner_id,id)'));
 assert.ok(!route.includes('body.owner_id'));
});
test('RAG-API-16 default database state draft',()=>assert.ok(rust.includes("'draft'")));
test('RAG-API-17 revision conditional update',()=>assert.ok(rust.includes('WHERE id=? AND revision=?')));
test('RAG-API-18 cloud connector requires HTTPS',()=>assert.ok(rust.includes('starts_with("https://")')));
test('RAG-UI-19 only one agent input shown',()=>assert.ok(source.includes("state.provider==='multica'")&&source.includes("state.provider==='orca'")));
