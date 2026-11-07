export const statuses = ['draft', 'approved', 'synced', 'archived'];
export const statusText = { draft: '待审核', approved: '已批准', synced: '已入库', archived: '已归档' };
export function cleanTags(value) {
    const raw = Array.isArray(value) ? value : String(value || '').split(/[,，]/);
    return [...new Set(raw.map(v => v.trim()).filter(v => v.length > 0 && v.length <= 32))].slice(0, 12);
}
export function splitContext(text, max = 850, overlap = 110) {
    const chars = Array.from((text || '').trim());
    if (!chars.length || max <= overlap)
        return [];
    let start = 0;
    const chunks = [];
    while (start < chars.length && chunks.length < 128) {
        const end = Math.min(start + max, chars.length);
        let cut = end;
        if (end < chars.length)
            for (let i = end - 1; i >= Math.min(end, start + Math.floor(max / 2)); i--) {
                if (/[。！？.\n]/.test(chars[i])) {
                    cut = i + 1;
                    break;
                }
            }
        if (cut <= start)
            cut = end;
        const chunk = chars.slice(start, cut).join('').trim();
        if (chunk)
            chunks.push(chunk);
        if (cut === chars.length)
            break;
        start = Math.max(start + 1, cut - overlap);
    }
    return chunks;
}
export function redactPreview(s) {
    return String(s).replace(/bearer\s+\S+/ig, '[REDACTED]')
        .replace(/(?:sk|ghp|gho|ghu|ghs|github_pat|AIza)[-_][\w-]{12,}|(?:api[_ -]?key|authorization|password|secret)\s*[:=]\s*\S+/ig, '[REDACTED]');
}
export function buildCitationContext(chunks) {
    return chunks.map(c => `[Argo-RAG:${c.id}] ${c.title}\n${c.content}`).join('\n\n---\n\n');
}
export function validScope(path) {
    return !path || (!/^[\\/]/.test(path) && !/[\0\\]/.test(path) && !path.split('/').includes('..') && !path.split('/').includes('.') && !/^[A-Za-z]:/.test(path));
}
export function canRunOnMachine(fileOwner, runner, repo) {
    return !repo || Boolean(fileOwner && runner);
}
//# sourceMappingURL=rag.lib.js.map