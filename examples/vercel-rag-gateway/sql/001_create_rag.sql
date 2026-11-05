-- Apply once to the remote Postgres database. pgvector is optional; lexical RAG works without it.
CREATE TABLE IF NOT EXISTS argo_rag_chunks (
  id TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  source_key TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  provider TEXT NOT NULL,
  project TEXT NOT NULL,
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  tags JSONB NOT NULL DEFAULT '[]',
  fingerprint TEXT NOT NULL,
  source_revision INT NOT NULL,
  embedding_status TEXT NOT NULL DEFAULT 'pending',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY(owner_id,id),
  UNIQUE(owner_id,fingerprint)
);
CREATE INDEX IF NOT EXISTS ix_argo_rag_project ON argo_rag_chunks(owner_id,project,provider,updated_at DESC);
CREATE INDEX IF NOT EXISTS ix_argo_rag_search ON argo_rag_chunks USING GIN (to_tsvector('simple', content));
-- Optional semantic extension (requires pgvector and a compatible 1536-d embedding model):
-- CREATE EXTENSION IF NOT EXISTS vector;
-- ALTER TABLE argo_rag_chunks ADD COLUMN embedding vector(1536);
-- CREATE INDEX ix_argo_rag_hnsw ON argo_rag_chunks USING hnsw (embedding vector_cosine_ops);
