/** TypeScript watch compilation + zero-dependency Node development HTTP server. */
import {spawn} from 'node:child_process';
import {createWebServer} from './serve-web.mjs';
const compiler=spawn(process.platform==='win32'?'npx.cmd':'npx',['tsc','-p','tsconfig.json','--watch','--preserveWatchOutput'],{stdio:'inherit'});
const server=createWebServer(8997);
function cleanup(){server.close();if(compiler.pid&&!compiler.killed)compiler.kill();}
compiler.on('exit',code=>{if(code&&code!==0){cleanup();process.exitCode=code}});
process.on('SIGINT',cleanup);process.on('SIGTERM',cleanup);
