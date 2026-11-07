/** Red-green regression gates discovered during the adversarial audit.
 * These are structural security contracts, NOT a substitute for compiling Rust.
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const load = p => readFileSync(new URL(p,import.meta.url),'utf8');
const worker=load('../crates/argo/src/worker.rs');
const server=load('../crates/argo/src/server.rs');
const model=load('../crates/argo/src/model.rs');
const ai=load('../crates/argo/src/ai.rs');
const rag=load('../crates/argo/src/rag.rs');
const desktop=load('../src-tauri/src/main.rs');
const ui=load('../src/app.ts');

test('ADV-01 backslash scope guard is a valid escaped Rust char literal',()=>{
  assert.ok(worker.includes("path.contains('\\\\')"),'scope guard must compile as a Rust backslash char');
  assert.ok(!worker.includes("path.contains('\\')"),'unescaped Rust char literal must be removed');
});
test('ADV-02 node identities are safe for path joins',()=>{
  assert.ok(model.includes('pub fn safe_node_id('),'central validation required');
  assert.ok(server.includes('safe_node_id(&n.id)'),'registration must validate node id');
  assert.ok(worker.includes('safe_node_id(&id)'),'local worker must reject unsafe node id');
});
test('ADV-03 each remote worker has node-bound credentials, legacy only with explicit opt-in',()=>{
  assert.ok(server.includes('ARGO_NODE_TOKENS_JSON'),'per-node credentials must be supported');
  assert.ok(server.includes('ARGO_ALLOW_SHARED_WORKER_TOKEN'),'legacy auth must be opted into');
  assert.ok(server.includes('auth_node('),'node-bound authentication required');
});
test('ADV-04 late events and artifacts cannot be accepted after lease expiry',()=>{
  assert.ok(server.includes("state IN ('running','cancelling') AND lease_until>?"),'worker ownership must enforce active lease');
});
test('ADV-05 AI apply requires a server-issued single-use preview token',()=>{
  assert.ok(ai.includes('rag_ai_previews'),'persist preview snapshots');
  assert.ok(ai.includes('preview_id'),'API must carry preview identity');
  assert.ok(ai.includes('consumed_at'),'preview token must be single use');
  assert.ok(ui.includes('preview_id:result.preview_id') || ui.includes('preview_id:result['),'UI must retain server preview identity');
});
test('ADV-06 blob worker auth is not unconditionally the cluster-wide shared secret',()=>{
  assert.ok(server.includes('auth_worker_any('),'node auth for blob endpoints');
});
test('ADV-07 repository mappings support path separators and semicolons via JSON',()=>{
  assert.ok(worker.includes('ARGO_REPOS_JSON'),'worker must parse JSON repo map');
  assert.ok(desktop.includes('ARGO_REPOS_JSON'),'desktop must send JSON mapping');
});
test('ADV-08 file scope needs a rule for prefix and symlink safety',()=>{
  assert.ok(worker.includes('scope_allowed(name,scope)'));
  assert.ok(worker.includes('kind.is_symlink()') || worker.includes('!kind.is_file()&&!kind.is_dir()'));
});
test('ADV-09 RAG restore and batch commits use optimistic revision checks',()=>{
  assert.ok(ai.includes('row.revision!=change.revision') || ai.includes('row.revision != change.revision'));
  assert.ok(ai.includes("status='draft'"));
});
test('ADV-10 desktop does not expose arbitrary shell or auth tokens to webview',()=>{
  assert.ok(desktop.includes('argo_request'));
  assert.ok(!ui.includes('ARGO_ADMIN_TOKEN'));
  assert.deepEqual(JSON.parse(load('../src-tauri/capabilities/default.json')).permissions,['core:default']);
});

test('ADV-11 patch approval refuses truncated and tampered diffs',()=>{
 const block=server.slice(server.indexOf('async fn approve_patch'),server.indexOf('async fn claim_apply'));
 assert.ok(block.includes('bytes.len()>200_000'));
 assert.ok(block.includes('Sha256::digest(&bytes)'));
});
test('ADV-12 task routing checks the named execution node capability',()=>{
 const block=server.slice(server.indexOf('async fn create_task'),server.indexOf('async fn events'));
 assert.ok(block.includes('execution node is offline'));
 assert.ok(block.includes('selected agent is unavailable'));
});
