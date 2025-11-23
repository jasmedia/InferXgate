-- Initial schema for LLM Gateway
-- Creates tables for usage tracking and analytics

-- Usage records table
CREATE TABLE IF NOT EXISTS usage_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model VARCHAR(255) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    cost_usd DOUBLE PRECISION NOT NULL,
    latency_ms BIGINT NOT NULL,
    user_id VARCHAR(255),
    cached BOOLEAN NOT NULL DEFAULT false,
    error TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_usage_records_created_at
ON usage_records(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_usage_records_model
ON usage_records(model);

CREATE INDEX IF NOT EXISTS idx_usage_records_provider
ON usage_records(provider);

CREATE INDEX IF NOT EXISTS idx_usage_records_user_id
ON usage_records(user_id)
WHERE user_id IS NOT NULL;

-- Composite index for common queries
CREATE INDEX IF NOT EXISTS idx_usage_records_model_provider
ON usage_records(model, provider, created_at DESC);

-- Comments for documentation
COMMENT ON TABLE usage_records IS 'Tracks all LLM API requests with usage and cost information';
COMMENT ON COLUMN usage_records.model IS 'The model name used for the request';
COMMENT ON COLUMN usage_records.provider IS 'The LLM provider (anthropic, gemini, openai, etc.)';
COMMENT ON COLUMN usage_records.prompt_tokens IS 'Number of tokens in the prompt';
COMMENT ON COLUMN usage_records.completion_tokens IS 'Number of tokens in the completion';
COMMENT ON COLUMN usage_records.total_tokens IS 'Total tokens (prompt + completion)';
COMMENT ON COLUMN usage_records.cost_usd IS 'Cost in USD for this request';
COMMENT ON COLUMN usage_records.latency_ms IS 'Request latency in milliseconds';
COMMENT ON COLUMN usage_records.user_id IS 'Optional user identifier';
COMMENT ON COLUMN usage_records.cached IS 'Whether this was served from cache';
COMMENT ON COLUMN usage_records.error IS 'Error message if request failed';
