/** Pure RAG helpers used by the UI and unit tests. Embeddings run remotely after review. */
export const statuses=['draft','approved','synced','archived'];
export const statusText={draft:'待审核',approved:'已批准',synced:'已入库',archived:'已归档'};
export function cleanTags(str){return [...new Set((Array.isArray(str)?str:String(str||'').split(/[,，]/)).map(v=>v.trim()).filter(v=>v&&v.length<=32))].slice(0,12)}
export function splitContext(text,max=850,overlap=110){
 const v=Array.from((text||'').trim());if(!v.length||max<=overlap)return [];
 let start=0;const out=[];while(start<v.length&&out.length<128){const end=Math.min(start+max,v.length);let cut=end;if(end<v.length){for(let i=end-1;i>=Math.min(end,start+Math.floor(max/2));i--){if(/[。！？.\n]/.test(v[i])){cut=i+1;break}}}if(cut<=start)cut=end;let part=v.slice(start,cut).join('').trim();if(part)out.push(part);if(cut===v.length)break;start=Math.max(start+1,cut-overlap)}return out;
}
export function redactPreview(s){return String(s).replace(/bearer\s+\S+/ig,'[REDACTED]').replace(/(?:sk|ghp|gho|ghu|ghs|github_pat|AIza)[-_][\w-]{12,}|(?:api[_ -]?key|authorization|password|secret)\s*[:=]\s*\S+/ig,'[REDACTED]')}

export function buildCitationContext(chunks){return chunks.map((c,i)=>`[Argo-RAG:${c.id}] ${c.title}\n${c.content}`).join('\n\n---\n\n')}
export function validScope(s){return !s||(!/^[\\/]/.test(s)&&!/[\0\\]/.test(s)&&!s.split('/').includes('..')&&!s.split('/').includes('.')&&!/^[A-Za-z]:/.test(s));}
export function canRunOnMachine(fileOwner,runner,repo){return !repo||!fileOwner||fileOwner===runner}
