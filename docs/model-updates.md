# Model Updates

InferXgate supports the latest AI models from Anthropic and Google Gemini.

## Recent Additions (Anthropic)

| Model | Model ID | Context | Pricing (per 1M tokens) |
|-------|----------|---------|------------------------|
| **Claude Opus 4.7** | `claude-opus-4-7` | 200K | $15 input / $75 output |
| **Claude Sonnet 4.6** | `claude-sonnet-4-6` | 200K | $3 input / $15 output |

Model IDs without a date suffix resolve to Anthropic's latest dated release for that line. Dated IDs (e.g. `claude-opus-4-5-20251101`) continue to work for version pinning.

## Supported Anthropic Models

- Claude Opus 4.7
- Claude Sonnet 4.6
- Claude Opus 4.5 (`claude-opus-4-5-20251101`)
- Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`)
- Claude Haiku 4.5 (`claude-haiku-4-5-20251001`)
- Claude Opus 4.1 (`claude-opus-4-1-20250805`)
- Claude Sonnet 4 (`claude-sonnet-4-20250514`)
- Claude Opus 4 (`claude-opus-4-20250514`)
- Claude 3.5 Haiku (`claude-3-5-haiku-20241022`)
- Claude 3 Haiku (`claude-3-haiku-20240307`)

## Supported Gemini Models

- Gemini 3 Pro Preview (`gemini-3-pro-preview`)
- Gemini 3 Pro Image Preview (`gemini-3-pro-image-preview`)
- Gemini 2.5 Pro (`gemini-2.5-pro`)
- Gemini 2.5 Flash (`gemini-2.5-flash`)
- Gemini 2.5 Flash Lite (`gemini-2.5-flash-lite`)
- Gemini 2.5 Flash Image (`gemini-2.5-flash-image`)
- Gemini 2.0 Flash (`gemini-2.0-flash`)
- Gemini 2.0 Flash Lite (`gemini-2.0-flash-lite`)

## How to Use

Specify the model ID in your API request:

```bash
curl -X POST https://your-gateway/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-7",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

All models are accessible through InferXgate's unified OpenAI-compatible API.
