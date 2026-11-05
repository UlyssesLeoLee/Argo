/** Tokens remain inside Rust: the frontend only invokes narrowly scoped commands. */
export function nativeInvoke() { return window.__TAURI__?.core?.invoke ?? null; }
export async function nativeRequest(path, method = 'GET', body) {
    if (!path.startsWith('/api/v1/') || path.includes('..'))
        throw new Error('Invalid API path');
    const invoke = nativeInvoke();
    if (!invoke)
        throw new Error('Tauri runtime unavailable');
    return invoke('argo_request', { path, method, body: body ?? null });
}
export async function nativeStatus() {
    const invoke = nativeInvoke();
    if (!invoke)
        throw new Error('Tauri runtime unavailable');
    return invoke('argo_daemon_status');
}
//# sourceMappingURL=native.js.map