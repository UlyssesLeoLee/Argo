import { NextRequest, NextResponse } from 'next/server';
import postgres from 'postgres';
import { timingSafeEqual } from 'node:crypto';
export const runtime='nodejs';
const sql=postgres(process.env.DATABASE_URL||'',{max:3,idle_timeout:10});
const owner=process.env.ARGO_RAG_OWNER_ID||'personal-owner';
export async function POST(request:NextRequest){
  const expected=process.env.ARGO_RAG_API_TOKEN||'';
  const actual=request.headers.get('authorization')?.replace(/^Bearer\s+/i,'')||'';
  const a=Buffer.from(actual),b=Buffer.from(expected);
  if(expected.length<20||a.length!==b.length||!timingSafeEqual(a,b))return NextResponse.json({error:'unauthorized'},{status:401});
  const {query,project,provider,limit=8}=await request.json().catch(()=>({}));
  if(typeof query!=='string'||!query.trim()||query.length>400)return NextResponse.json({error:'invalid_query'},{status:400});
  const search='%'+query.replace(/[%_\\]/g,c=>'\\'+c)+'%';
  const rows=await sql`SELECT id,title,content,provider,project,tags,source_key
    FROM argo_rag_chunks WHERE owner_id=${owner} AND (title ILIKE ${search} ESCAPE '\\' OR content ILIKE ${search} ESCAPE '\\')
    AND (${project||null}::text IS NULL OR project=${project||null})
    AND (${provider||null}::text IS NULL OR provider=${provider||null})
    ORDER BY updated_at DESC LIMIT ${Math.min(Math.max(Number(limit)||8,1),20)}`;
  return NextResponse.json({results:rows,mode:'keyword',embedding_search:false});
}
