// 临时本地 dev AI 网关（PI-GW-DEV-01）。
//
// ⚠️ 人类 2026-06-26 明确授权临时偏离云 APC「云代码开发启动迁出独立仓库」约定（EXEMPTION-GW-DEV-01）。
// 正式版仍迁独立仓库、改 TypeScript、补 BYOK 加密(GW-02)/AUTH/LIC/真实 CLOUD-GATE/内容不留存审计。
// 本文件为零依赖 Node ESM dev 脚手架：让客户端连上 localhost、用用户 key 跑通真实流式 AI，替换桩回复。
//
// 安全：密钥仅取自请求体(BYOK)或环境变量(平台)，**不落盘、不写日志**（REQ-GW-006 dev 落实）。

import { createServer } from 'node:http';

const PORT = Number(process.env.GW_PORT) || 8787;

// 供应商适配（REQ-GW-001）：openai/deepseek 共享 OpenAI 兼容；anthropic 独立 Messages API。
const PROVIDERS = {
  openai: { kind: 'openai', url: 'https://api.openai.com/v1/chat/completions' },
  deepseek: { kind: 'openai', url: 'https://api.deepseek.com/v1/chat/completions' },
  anthropic: { kind: 'anthropic', url: 'https://api.anthropic.com/v1/messages' },
};

// BYOK 内存代持（PI-BYOK-DEV-01，dev 单用户）：{ provider: key }。仅内存，不落盘、不记日志。
const byokKeys = {};

function sse(res, event, data) {
  res.write(`event: ${event}\n`);
  res.write(`data: ${JSON.stringify(data)}\n\n`);
}

async function readJson(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  if (!chunks.length) return {};
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

// 通用 provider SSE 读取：逐行解析 `data: {...}`，跳过 [DONE] 与心跳。
async function pumpProviderSse(resp, onJson) {
  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buf = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });
    let nl;
    while ((nl = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line.startsWith('data:')) continue;
      const payload = line.slice(5).trim();
      if (!payload || payload === '[DONE]') continue;
      try {
        onJson(JSON.parse(payload));
      } catch {
        /* 非 JSON 心跳，忽略 */
      }
    }
  }
}

// OpenAI 兼容（OpenAI + DeepSeek）：Bearer + chat/completions 流式。
async function streamOpenAiCompat(provider, { model, messages, apiKey, tools }, res) {
  const reqBody = { model, messages, stream: true };
  if (Array.isArray(tools) && tools.length) reqBody.tools = tools; // function-calling（PI-AG-08B）
  const resp = await fetch(provider.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiKey}` },
    body: JSON.stringify(reqBody),
  });
  if (!resp.ok) {
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  // 累积流式 tool_calls（OpenAI/DeepSeek：按 index 拼接 id/name/arguments 片段）。
  const toolAcc = {};
  await pumpProviderSse(resp, (json) => {
    const choice = json.choices?.[0];
    if (!choice) return;
    const delta = choice.delta?.content;
    if (delta) sse(res, 'chunk', { text: delta });
    const tcs = choice.delta?.tool_calls;
    if (Array.isArray(tcs)) {
      for (const tc of tcs) {
        const i = tc.index ?? 0;
        toolAcc[i] = toolAcc[i] || { id: '', name: '', arguments: '' };
        if (tc.id) toolAcc[i].id = tc.id;
        if (tc.function?.name) toolAcc[i].name = tc.function.name;
        if (tc.function?.arguments) toolAcc[i].arguments += tc.function.arguments;
      }
    }
    const finish = choice.finish_reason;
    if (finish === 'tool_calls') {
      sse(res, 'tool_calls', { tool_calls: Object.values(toolAcc) }); // 交客户端执行（工具在客户端）
    } else if (finish) {
      sse(res, 'done', { stop_reason: finish });
    }
  });
}

// OpenAI 格式 messages → Anthropic（system 抽顶层；tool_calls→tool_use；role:tool→user tool_result）。
function toAnthropicMessages(messages) {
  let system = '';
  const out = [];
  for (const m of messages) {
    if (m.role === 'system') {
      system += (system ? '\n\n' : '') + (m.content ?? '');
    } else if (m.role === 'assistant' && Array.isArray(m.tool_calls)) {
      const content = [];
      if (m.content) content.push({ type: 'text', text: m.content });
      for (const tc of m.tool_calls) {
        let input = {};
        try { input = JSON.parse(tc.function?.arguments || '{}'); } catch { input = {}; }
        content.push({ type: 'tool_use', id: tc.id, name: tc.function?.name, input });
      }
      out.push({ role: 'assistant', content });
    } else if (m.role === 'tool') {
      out.push({ role: 'user', content: [{ type: 'tool_result', tool_use_id: m.tool_call_id, content: m.content ?? '' }] });
    } else {
      out.push({ role: m.role, content: m.content ?? '' });
    }
  }
  return { system, messages: out };
}

// OpenAI tools → Anthropic tools（input_schema）。
function toAnthropicTools(tools) {
  if (!Array.isArray(tools) || !tools.length) return undefined;
  return tools.map((t) => ({
    name: t.function?.name,
    description: t.function?.description,
    input_schema: t.function?.parameters ?? { type: 'object', properties: {} },
  }));
}

// Anthropic Messages API：x-api-key + anthropic-version；请求/tools 翻译 + tool_use 流式解析（PI-GW-DEV-02）。
async function streamAnthropic(provider, { model, messages, apiKey, maxTokens, tools }, res) {
  const { system, messages: amsgs } = toAnthropicMessages(messages);
  const body = { model, messages: amsgs, stream: true, max_tokens: maxTokens || 4096 };
  if (system) body.system = system;
  const atools = toAnthropicTools(tools);
  if (atools) body.tools = atools;

  const resp = await fetch(provider.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
    },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  // 累积 tool_use（按 content block index：start 取 id/name，input_json_delta 累积参数）。
  const toolAcc = {};
  await pumpProviderSse(resp, (json) => {
    if (json.type === 'content_block_start' && json.content_block?.type === 'tool_use') {
      toolAcc[json.index] = { id: json.content_block.id, name: json.content_block.name, arguments: '' };
    } else if (json.type === 'content_block_delta') {
      if (json.delta?.type === 'text_delta') {
        sse(res, 'chunk', { text: json.delta.text });
      } else if (json.delta?.type === 'input_json_delta' && toolAcc[json.index]) {
        toolAcc[json.index].arguments += json.delta.partial_json || '';
      }
    } else if (json.type === 'message_delta' && json.delta?.stop_reason) {
      const stop = json.delta.stop_reason;
      if (stop === 'tool_use') {
        const arr = Object.values(toolAcc).map((t) => ({ id: t.id, name: t.name, arguments: t.arguments || '{}' }));
        sse(res, 'tool_calls', { tool_calls: arr }); // 归一化，与 OpenAI 适配器同形
      } else {
        sse(res, 'done', { stop_reason: stop });
      }
    }
  });
}

const server = createServer(async (req, res) => {
  // dev CORS（Tauri/前端本地联调）。
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  if (req.method === 'GET' && req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, dev: true, providers: Object.keys(PROVIDERS) }));
    return;
  }

  // 最小门禁（dev 恒通过）；真实订阅裁决归服务端 GW+AUTH+LIC（X-FORBID-002），正式版补。
  if (req.method === 'POST' && req.url === '/gate/check') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ verdict: 'passed', dev: true }));
    return;
  }

  // BYOK 代持（PI-BYOK-DEV-01）：客户端一次性上送 provider 密钥，GW 内存保管（dev）。不回显明文。
  if (req.method === 'POST' && req.url === '/byok/keys') {
    let body;
    try {
      body = await readJson(req);
    } catch {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'invalid json body' }));
      return;
    }
    const name = body.provider;
    if (!PROVIDERS[name] || !body.api_key) {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'require { provider(openai|deepseek|anthropic), api_key }' }));
      return;
    }
    byokKeys[name] = body.api_key;
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, provider: name })); // 不回显密钥
    return;
  }

  if (req.method === 'POST' && req.url === '/invoke/stream') {
    let body;
    try {
      body = await readJson(req);
    } catch {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'invalid json body' }));
      return;
    }
    const name = body.provider || 'openai';
    const provider = PROVIDERS[name];
    if (!provider) {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: `unknown provider: ${name}`, providers: Object.keys(PROVIDERS) }));
      return;
    }
    // 密钥优先级：请求体 api_key > BYOK 代持 > 平台 env GW_<PROVIDER>_KEY。不落盘、不记录。
    const apiKey = body.api_key || byokKeys[name] || process.env[`GW_${name.toUpperCase()}_KEY`];
    if (!apiKey) {
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: `no api key for ${name} (传 body.api_key 或设 env GW_${name.toUpperCase()}_KEY)` }));
      return;
    }
    if (!body.model || !Array.isArray(body.messages)) {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'require { model, messages[] }' }));
      return;
    }

    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });
    const args = { model: body.model, messages: body.messages, apiKey, maxTokens: body.max_tokens, tools: body.tools };
    try {
      if (provider.kind === 'anthropic') await streamAnthropic(provider, args, res);
      else await streamOpenAiCompat(provider, args, res);
    } catch (e) {
      sse(res, 'error', { code: 'GW_STREAM_ERR', message: String(e?.message || e), retriable: true });
    }
    res.end();
    return;
  }

  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, () => {
  // 仅打印端口与供应商名，绝不打印任何密钥。
  console.log(`[dev-gateway] listening on http://localhost:${PORT} — providers: ${Object.keys(PROVIDERS).join(', ')}`);
});
