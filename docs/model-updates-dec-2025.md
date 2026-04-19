# Model Updates

InferXgate supports the latest AI models from Anthropic and Google Gemini.

## New Models Added

### Anthropic Claude

| Model | Model ID | Context | Pricing (per 1M tokens) |
|-------|----------|---------|------------------------|
| **Claude Opus 4.7** | `claude-opus-4-7` | 200K | $15 input / $75 output |
| **Claude Sonnet 4.6** | `claude-sonnet-4-6` | 200K | $3 input / $15 output |
| Claude Opus 4.5 | `claude-opus-4-5-20251101` | 200K | $15 input / $75 output |

Claude Opus 4.7 is Anthropic's most capable model as of April 2026, succeeding Opus 4.5 with improved reasoning and agentic performance. Claude Sonnet 4.6 offers the best balance between capability and cost.

### Google Gemini

| Model | Model ID | Context | Pricing (per 1M tokens) |
|-------|----------|---------|------------------------|
| **Gemini 3 Pro** | `gemini-3-pro-preview` | 1M | $1.25 input / $5.00 output |

Gemini 3 Pro is Google's latest flagship model, described as "the best model in the world for multimodal understanding." Released in November 2025, it supports text, image, video, audio, and PDF inputs.

## Updated Pricing

We've updated pricing for all current Gemini models:

| Model | Model ID | Input | Output |
|-------|----------|-------|--------|
| Gemini 3 Pro | `gemini-3-pro-preview` | $1.25 | $5.00 |
| Gemini 2.5 Pro | `gemini-2.5-pro` | $1.25 | $5.00 |
| Gemini 2.5 Flash | `gemini-2.5-flash` | $0.075 | $0.30 |
| Gemini 2.5 Flash Lite | `gemini-2.5-flash-lite` | $0.075 | $0.30 |
| Gemini 2.0 Flash | `gemini-2.0-flash` | $0.10 | $0.40 |

## Full Model Support

### Anthropic Claude Models
- Claude Opus 4.7 (NEW)
- Claude Sonnet 4.6 (NEW)
- Claude Opus 4.5
- Claude Sonnet 4.5
- Claude Haiku 4.5
- Claude Opus 4.1
- Claude Sonnet 4
- Claude Opus 4
- Claude 3.5 Haiku
- Claude 3 Haiku

### Google Gemini Models
- Gemini 3 Pro Preview (NEW in primary routing)
- Gemini 3 Pro Image Preview
- Gemini 2.5 Pro
- Gemini 2.5 Flash
- Gemini 2.5 Flash Lite
- Gemini 2.5 Flash Image
- Gemini 2.0 Flash
- Gemini 2.0 Flash Lite

## How to Use

Simply specify the model ID in your API requests:

```bash
curl -X POST https://your-gateway/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-7",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

Or for Gemini:

```bash
curl -X POST https://your-gateway/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-3-pro-preview",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

All models are accessible through InferXgate's unified OpenAI-compatible API.
