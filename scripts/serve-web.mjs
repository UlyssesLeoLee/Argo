/** A zero-dependency local development/preview server. Not shipped as a backend. */
import {createServer} from 'node:http';
import {readFile,stat} from 'node:fs/promises';
import {resolve,dirname,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../web');
const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.svg':'image/svg+xml','.png':'image/png','.jpg':'image/jpeg','.ico':'image/x-icon','.json':'application/json'};
export function createWebServer(port=8899){
 const server=createServer(async(req,res)=>{
   try{
     const url=new URL(req.url??'/',`http://${req.headers.host??'127.0.0.1'}`);
     const rel=decodeURIComponent(url.pathname).replaceAll('\\','/');
     if(rel.includes('\0') || rel.split('/').includes('..')){res.writeHead(403);res.end('Forbidden');return}
     const path=resolve(root,'.'+(rel==='/'?'/index.html':rel));
     if(!path.startsWith(root+sep)){res.writeHead(403);res.end('Forbidden');return}
     const info=await stat(path).catch(()=>null);
     if(!info?.isFile()){res.writeHead(404);res.end('Not found');return}
     const ext=path.slice(path.lastIndexOf('.'));
     const bytes=await readFile(path);
     res.writeHead(200,{'Content-Type':mime[ext]??'application/octet-stream','X-Content-Type-Options':'nosniff','Cache-Control':'no-store'});
     if(req.method==='HEAD')res.end();else res.end(bytes);
   }catch{res.writeHead(400);res.end('Bad request')}
 });
 server.listen(port,'127.0.0.1',()=>console.log(`Argo UI preview: http://127.0.0.1:${port}/?demo=1`));
 return server;
}
if(process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url))createWebServer(Number(process.env.ARGO_WEB_PORT)||8899);
