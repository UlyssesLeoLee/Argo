import type {NativeInvoke,NativeDaemonStatus,ApiMethod} from './contracts.js';
declare global {interface Window {__TAURI__?:{core:{invoke:NativeInvoke}}}}
/** Tokens remain inside Rust: the frontend only invokes narrowly scoped commands. */
export function nativeInvoke():NativeInvoke|null {return window.__TAURI__?.core?.invoke??null;}
export async function nativeRequest<T>(path:string,method:ApiMethod='GET',body?:unknown):Promise<T>{
  if(!path.startsWith('/api/v1/')||path.includes('..'))throw new Error('Invalid API path');
  const invoke=nativeInvoke();if(!invoke)throw new Error('Tauri runtime unavailable');
  return invoke<T>('argo_request',{path,method,body:body??null});
}
export async function nativeStatus():Promise<NativeDaemonStatus>{
  const invoke=nativeInvoke();if(!invoke)throw new Error('Tauri runtime unavailable');
  return invoke<NativeDaemonStatus>('argo_daemon_status');
}
