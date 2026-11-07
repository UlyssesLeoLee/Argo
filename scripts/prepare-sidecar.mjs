/** Build the real Rust daemon and copy it to Tauri v2's target-triple sidecar name. */
import {execFileSync} from 'node:child_process';
import {copyFileSync,mkdirSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
const isRelease=process.argv.includes('--release');
const host=execFileSync('rustc',['-vV'],{encoding:'utf8'}).match(/^host:\s*(\S+)/m)?.[1];
if(!host)throw Error('rustc host target triple unavailable. Install Rust stable first.');
execFileSync('cargo',['build','-p','argo-runtime',...(isRelease?['--release']:[])],{cwd:root,stdio:'inherit'});
const extension=process.platform==='win32'?'.exe':'';
const source=join(root,'target',isRelease?'release':'debug',`argo-runtime${extension}`);
const destination=join(root,'src-tauri','binaries',`argo-runtime-${host}${extension}`);
mkdirSync(join(root,'src-tauri','binaries'),{recursive:true});
copyFileSync(source,destination);
console.log(`Tauri sidecar prepared: ${destination}`);
