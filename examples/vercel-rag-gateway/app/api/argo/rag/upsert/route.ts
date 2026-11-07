import { NextRequest, NextResponse } from 'next/server';
import postgres from 'postgres';
import { timingSafeEqual } from 'node:crypto';

export const runtime = 'nodejs';
const sql = postgres(process.env.DATABASE_URL || '', { max: 3, idle_timeout: 10 });
const owner = process.env.ARGO_RAG_OWNER_ID || 'personal-owner';
function authorized(request: NextRequest) {
  const expected = process.env.ARGO_RAG_API_TOKEN || '';
  const received = request.headers.get('authorization')?.replace(/^Bearer\s+/i, '') || '';
  const a = Buffer.from(received);const b = Buffer.from(expected);
  return expected.length >= 20 && a.length === b.length && timingSafeEqual(a,b);
}
export async function POST(request: NextRequest) {
  if (!authorized(request)) return NextResponse.json({error:'unauthorized'},{status:401});
  const body=await request.json().catch(()=>null);
  if (!body || typeof body.id!=='string' || !/^rag-[a-f0-9]{24}$/.test(body.id)
    || typeof body.content!=='string' || !body.content.trim() || body.content.length>6000
    || typeof body.fingerprint!=='string' || !/^[a-f0-9]{64}$/.test(body.fingerprint)
    || typeof body.title!=='string' || body.title.length>240
    || !Array.isArray(body.tags) || body.tags.length>12) {
      return NextResponse.json({error:'invalid_payload'},{status:400});
  }
  const id = body.id;const source_key=String(body.source_key||'').slice(0,240);
  const source_kind=String(body.source_kind||'manual').slice(0,60);
  const provider=String(body.provider||'unknown').slice(0,40);
  const project=String(body.project||'Personal').slice(0,100);
  const tags=body.tags.map((t:unknown)=>String(t).slice(0,32));
  const revision=Number.isInteger(body.revision)?body.revision:1;
  try {
    // Idempotent upsert, using trusted owner from the environment rather than the client payload.
    await sql`INSERT INTO argo_rag_chunks (id,owner_id,source_key,source_kind,provider,project,title,content,tags,fingerprint,source_revision,embedding_status)
      VALUES (${id},${owner},${source_key},${source_kind},${provider},${project},${body.title},${body.content},${sql.json(tags)},${body.fingerprint},${revision},'pending')
      ON CONFLICT(owner_id,id) DO UPDATE SET title=EXCLUDED.title,content=EXCLUDED.content,tags=EXCLUDED.tags,
      source_revision=EXCLUDED.source_revision,embedding_status='pending',updated_at=NOW()
      WHERE argo_rag_chunks.source_revision <= EXCLUDED.source_revision`;
    return NextResponse.json({id,status:'stored',embedding_status:'pending'});
  } catch (error) {
    console.error('Argo RAG ingestion failed',error instanceof Error?error.name:'db_failure');
    return NextResponse.json({error:'database_failure'},{status:503});
  }
}
