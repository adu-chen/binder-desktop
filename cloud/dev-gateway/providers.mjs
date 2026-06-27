// 供应商适配 + 归一化流式（PI-GW-DEV-03 从 server.mjs 拆出）。
// openai/deepseek 共享 OpenAI 兼容；anthropic 独立 Messages API（含请求/tools 翻译 + tool_use 解析）。

export const PROVIDERS = {
  openai: { kind: 'openai', url: 'https://api.openai.com/v1/chat/completions' },
  deepseek: { kind: 'openai', url: 'https://api.deepseek.com/v1/chat/completions' },
  anthropic: { kind: 'anthropic', url: 'https://api.anthropic.com/v1/messages' },
};

export function sse(res, event, data) {
  res.write(`event: ${event}\n`);
  res.write(`data: ${JSON.stringify(data)}\n\n`);
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

// OpenAI 兼容（OpenAI + DeepSeek）：Bearer + chat/completions 流式 + tool_calls 累积。
async function streamOpenAiCompat(provider, { model, messages, apiKey, tools }, res) {
  const reqBody = { model, messages, stream: true };
  if (Array.isArray(tools) && tools.length) reqBody.tools = tools;
  const resp = await fetch(provider.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiKey}` },
    body: JSON.stringify(reqBody),
  });
  if (!resp.ok) {
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  const toolAcc = {};
  await pumpProviderSse(resp, (json) => {
    const choice = json.choices?.[0];
    if (!choice) return;
    if (choice.delta?.content) sse(res, 'chunk', { text: choice.delta.content });
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
    if (finish === 'tool_calls') sse(res, 'tool_calls', { tool_calls: Object.values(toolAcc) });
    else if (finish) sse(res, 'done', { stop_reason: finish });
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

function toAnthropicTools(tools) {
  if (!Array.isArray(tools) || !tools.length) return undefined;
  return tools.map((t) => ({
    name: t.function?.name,
    description: t.function?.description,
    input_schema: t.function?.parameters ?? { type: 'object', properties: {} },
  }));
}

// Anthropic Messages API：请求/tools 翻译 + tool_use 流式解析。
async function streamAnthropic(provider, { model, messages, apiKey, maxTokens, tools }, res) {
  const { system, messages: amsgs } = toAnthropicMessages(messages);
  const body = { model, messages: amsgs, stream: true, max_tokens: maxTokens || 4096 };
  if (system) body.system = system;
  const atools = toAnthropicTools(tools);
  if (atools) body.tools = atools;

  const resp = await fetch(provider.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'x-api-key': apiKey, 'anthropic-version': '2023-06-01' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  const toolAcc = {};
  await pumpProviderSse(resp, (json) => {
    if (json.type === 'content_block_start' && json.content_block?.type === 'tool_use') {
      toolAcc[json.index] = { id: json.content_block.id, name: json.content_block.name, arguments: '' };
    } else if (json.type === 'content_block_delta') {
      if (json.delta?.type === 'text_delta') sse(res, 'chunk', { text: json.delta.text });
      else if (json.delta?.type === 'input_json_delta' && toolAcc[json.index]) toolAcc[json.index].arguments += json.delta.partial_json || '';
    } else if (json.type === 'message_delta' && json.delta?.stop_reason) {
      const stop = json.delta.stop_reason;
      if (stop === 'tool_use') sse(res, 'tool_calls', { tool_calls: Object.values(toolAcc) });
      else sse(res, 'done', { stop_reason: stop });
    }
  });
}

/** 按 provider.kind 路由到对应适配器，输出归一化 SSE。 */
export async function streamProvider(provider, args, res) {
  if (provider.kind === 'anthropic') await streamAnthropic(provider, args, res);
  else await streamOpenAiCompat(provider, args, res);
}
