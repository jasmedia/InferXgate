export const CONFIG = {
  INFERXGATE_URL: __ENV.INFERXGATE_URL || 'http://localhost:3000',
  LITELLM_URL: __ENV.LITELLM_URL || 'http://localhost:4000',
  MODEL: __ENV.TEST_MODEL || 'claude-sonnet-4-5-20250929',
  MAX_TOKENS: parseInt(__ENV.TEST_MAX_TOKENS) || 100,
  TEMPERATURE: parseFloat(__ENV.TEST_TEMPERATURE) || 0.7,
  LITELLM_API_KEY: __ENV.LITELLM_MASTER_KEY || 'sk-benchmark-litellm-key',
  TARGET: __ENV.TARGET || 'inferxgate',
};

export function getTargetUrl() {
  return CONFIG.TARGET === 'litellm' ? CONFIG.LITELLM_URL : CONFIG.INFERXGATE_URL;
}

export function getHeaders() {
  const headers = { 'Content-Type': 'application/json' };
  if (CONFIG.TARGET === 'litellm') {
    headers['Authorization'] = `Bearer ${CONFIG.LITELLM_API_KEY}`;
  }
  return headers;
}

export function getChatPayload(promptText) {
  return JSON.stringify({
    model: CONFIG.MODEL,
    messages: [{ role: 'user', content: promptText || 'What is 2 + 2?' }],
    max_tokens: CONFIG.MAX_TOKENS,
    temperature: CONFIG.TEMPERATURE,
    stream: false
  });
}

export const TEST_PROMPTS = [
  'What is 2 + 2? Answer with just the number.',
  'Explain the concept of recursion in one sentence.',
  'Write a haiku about programming.',
  'List three benefits of cloud computing.',
  'What is the capital of France?'
];
