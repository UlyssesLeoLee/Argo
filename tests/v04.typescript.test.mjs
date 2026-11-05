import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,existsSync,readdirSync} from 'node:fs';
const get=p=>readFileSync(new URL(p,import.meta.url),'utf8');
const manifest=JSON.parse(get('../package.json'));
const config=JSON.parse(get('../tsconfig.json'));
const strict=JSON.parse(get('../tsconfig.contracts.json'));
const tauri=JSON.parse(get('../src-tauri/tauri.conf.json'));
const source=get('../src/app.ts'),dom=get('../web/index.html');

test('TS-01 frontend source is TypeScript, not handwritten JS',()=>{
  assert.ok(existsSync(new URL('../src/app.ts',import.meta.url)));
  assert.ok(existsSync(new URL('../src/lib.ts',import.meta.url)));
  assert.ok(existsSync(new URL('../src/rag.lib.ts',import.meta.url)));
  assert.equal(existsSync(new URL('../web/app.mjs',import.meta.url)),false);
});
test('TS-02 browser loads ES module output built from TS',()=>{
  assert.ok(dom.includes('src="./app.js"'));
  assert.ok(get('../web/app.js').includes("./lib.js"));
  assert.equal(config.compilerOptions.module,'ES2022');
});
test('TS-03 core task and platform contracts are explicitly typed',()=>{
  const text=get('../src/contracts.ts');
  for(const name of ['AgentId','TaskCreate','InputContext','NodeSummary','RagChunk','NativeDaemonStatus'])assert.ok(text.includes(name));
  assert.ok(get('../src/lib.ts').includes('export function validateTask(p:TaskCreate)'));
});
test('TS-04 TypeScript is a required build step',()=>{
  assert.ok(manifest.scripts['build:web'].includes('tsc'));
  assert.ok(manifest.scripts.typecheck.includes('tsc'));
  assert.ok(manifest.devDependencies.typescript);
  assert.equal(config.compilerOptions.noEmitOnError,true);
});
test('TS-05 critical boundary definitions have strict typecheck',()=>{
  assert.equal(strict.compilerOptions.strict,true);
  for(const part of ['contracts.ts','native.ts','lib.ts','rag.lib.ts'])assert.ok(strict.include.some(x=>x.endsWith(part)));
});
test('TS-06 Tauri IPC does not hand out administrator secret',()=>{
  assert.ok(get('../src/native.ts').includes('nativeRequest<T>'));
  assert.ok(source.includes('nativeRequest(path,method,body)'));
  assert.ok(!source.includes('ARGO_ADMIN_TOKEN'));
});
test('TS-07 production Tauri build runs TS compile',()=>{
  assert.equal(tauri.build.beforeBuildCommand,'npm run build:web');
  assert.equal(tauri.bundle.active,true);
});
test('TS-08 Rust workspace and desktop sidecar are integrated',()=>{
  const c=get('../Cargo.toml');
  assert.ok(c.includes('src-tauri'));
  assert.deepEqual(tauri.bundle.externalBin,['binaries/argo-runtime']);
  assert.ok(get('../src-tauri/src/main.rs').includes('start_worker'));
});
test('TS-09 seven distinct platform types, STAR placeholder',()=>{
  const source=get('../src/contracts.ts');
  for(const id of ['claude','codex','hermes','orca','minimax','multica','star'])assert.ok(source.includes(`'${id}'`));
  assert.ok(get('../src/lib.ts').includes("symbol:'?'"));
});
test('TS-10 generic RAG and editable AI memory workflow remain visible',()=>{
  assert.ok(!/vercel/i.test(source+dom));
  for(const id of ['aiPreviewButton','aiApplyButton','ragConnector','ragSearch'])assert.ok(dom.includes(`id="${id}"`));
});
