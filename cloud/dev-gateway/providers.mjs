// 供应商适配 + 归一化流式（PI-GW-DEV-03 从 server.mjs 拆出）。
// openai/deepseek 共享 OpenAI 兼容；anthropic 独立 Messages API（含请求/tools 翻译 + tool_use 解析）。
import { createHash } from 'node:crypto';

export const PROVIDERS = {
  openai: { kind: 'openai', url: 'https://api.openai.com/v1/chat/completions' },
  deepseek: { id: 'deepseek', kind: 'openai', url: 'https://api.deepseek.com/v1/chat/completions' },
  anthropic: { kind: 'anthropic', url: 'https://api.anthropic.com/v1/messages' },
};

export function sse(res, event, data) {
  res.write(`event: ${event}\n`);
  res.write(`data: ${JSON.stringify(data)}\n\n`);
}

// Error names and machine codes are diagnostic facts; messages and stacks may contain
// request data and must never enter the gateway trace.
function streamFailureFacts(error) {
  const safeName = (value) => typeof value === 'string' && /^[A-Za-z][A-Za-z0-9]{0,63}$/.test(value) ? value : null;
  const safeCode = (value) => typeof value === 'string' && /^[A-Z][A-Z0-9_]{0,63}$/.test(value) ? value : null;
  const safeReason = (value) => typeof value === 'string' && /^[a-z][a-z_]{0,63}$/.test(value) ? value : null;
  return {
    errorName: safeName(error?.name) ?? 'Error',
    errorCode: safeCode(error?.code),
    reasonCode: safeReason(error?.reasonCode),
    causeName: safeName(error?.cause?.name),
    causeCode: safeCode(error?.cause?.code),
  };
}

function rejectProviderField(reasonCode) {
  const error = new Error(`Provider response protocol rejected: ${reasonCode}`);
  error.code = 'GW_PROVIDER_PROTOCOL';
  error.reasonCode = reasonCode;
  throw error;
}

// Parse complete SSE events. Only comments/empty events and [DONE] are heartbeats;
// malformed data and callback failures must never silently remove argument deltas.
async function pumpProviderSse(resp, onJson, onStage) {
  const reader = resp.body.getReader();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  let buf = '';
  let data = [];
  let frameOrdinal = 0;
  let firstByteSeen = false;
  const reject = (reasonCode) => {
    onStage?.('upstream_frame_rejected', { reasonCode, frameOrdinal });
    const error = new Error(`Provider SSE protocol rejected: ${reasonCode}`);
    error.code = 'GW_PROVIDER_PROTOCOL';
    throw error;
  };
  const dispatch = () => {
    if (!data.length) return;
    frameOrdinal++;
    const payload = data.join('\n');
    data = [];
    if (!payload.trim() || payload.trim() === '[DONE]') return;
    let json;
    try { json = JSON.parse(payload); }
    catch { reject('invalid_json_frame'); }
    // Deliberately outside JSON.parse's catch: a processing error is not a ping.
    try { onJson(json); }
    catch (error) {
      onStage?.('upstream_frame_processing_failed', { frameOrdinal, ...streamFailureFacts(error) });
      throw error;
    }
  };
  try { for (;;) {
    let next;
    try { next = await reader.read(); }
    catch (error) {
      onStage?.('upstream_body_read_failed', { frameOrdinal, ...streamFailureFacts(error) });
      throw error;
    }
    const { done, value } = next;
    if (done) {
      try { buf += decoder.decode(); } catch { reject('invalid_utf8'); }
      if (buf.trim() || data.length) reject('incomplete_event');
      break;
    }
    if (!firstByteSeen && value?.length) {
      firstByteSeen = true;
      onStage?.('upstream_first_byte');
    }
    try { buf += decoder.decode(value, { stream: true }); }
    catch { reject('invalid_utf8'); }
    let nl;
    while ((nl = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, nl).replace(/\r$/, '');
      buf = buf.slice(nl + 1);
      if (!line) { dispatch(); continue; }
      if (!line.startsWith('data:')) continue;
      data.push(line.slice(5).replace(/^ /, ''));
    }
  } } finally { reader.releaseLock(); }
}

function toolArgumentsObservation(call, index) {
  return { toolIndex: index, toolCallId: call.id, toolName: call.name,
    argumentsUtf8Bytes: Buffer.byteLength(call.arguments),
    argumentsSha256: createHash('sha256').update(call.arguments).digest('hex') };
}

function outputLimit(maxTokens) {
  if (maxTokens == null) return undefined;
  if (!Number.isSafeInteger(maxTokens) || maxTokens < 1) {
    const error = new Error('max_tokens must be a positive safe integer');
    error.code = 'GW_REQUEST_INVALID';
    throw error;
  }
  return maxTokens;
}

// Only numeric accounting crosses this boundary, never provider text or credentials.
function safeUsage(usage) {
  if (!usage || typeof usage !== 'object') return null;
  const count = (value) => Number.isSafeInteger(value) && value >= 0 ? value : null;
  return {
    promptTokens: count(usage.prompt_tokens ?? usage.input_tokens),
    completionTokens: count(usage.completion_tokens ?? usage.output_tokens),
    totalTokens: count(usage.total_tokens),
    reasoningTokens: count(usage.completion_tokens_details?.reasoning_tokens),
  };
}

function pendingArguments(toolAcc) {
  return Object.values(toolAcc).map((call, index) => {
    let jsonValid = false;
    try { JSON.parse(call.arguments); jsonValid = true; }
    catch { jsonValid = false; /* The emitted false is the diagnostic; never log parser text. */ }
    return { toolIndex: index, argumentsUtf8Bytes: Buffer.byteLength(call.arguments),
      argumentsSha256: createHash('sha256').update(call.arguments).digest('hex'), jsonValid };
  });
}


// One connect-level retry: a transient TCP/proxy connect failure must not surface as a
// failed model turn. Retries happen before any response byte is forwarded.
async function fetchUpstream(url, init, { signal, onStage }) {
  for (let attempt = 1; ; attempt += 1) {
    try {
      return await fetch(url, init);
    } catch (error) {
      const code = error?.cause?.code ?? null;
      const retriable = error?.name === 'TypeError'
        && (code === 'UND_ERR_CONNECT_TIMEOUT' || code === 'ECONNRESET' || code === null);
      if (signal?.aborted || attempt >= 2 || !retriable) throw error;
      onStage?.('upstream_connect_retry', { attempt, causeCode: code });
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
  }
}

// OpenAI 兼容（OpenAI + DeepSeek）：Bearer + chat/completions 流式 + tool_calls 累积。
async function streamOpenAiCompat(provider, { model, messages, apiKey, tools, maxTokens }, res, { signal, onStage }) {
  const reqBody = { model, messages, stream: true, stream_options: { include_usage: true } };
  // DeepSeek Flash defaults to thinking. This runtime does not replay private
  // reasoning_content on subsequent tool rounds, so explicitly use the
  // documented non-thinking mode. Do not rewrite legacy saved model names.
  if (provider.id === 'deepseek' && model === 'deepseek-flash') {
    reqBody.thinking = { type: 'disabled' };
  }
  const requestedMaxTokens = outputLimit(maxTokens);
  if (requestedMaxTokens !== undefined) reqBody.max_tokens = requestedMaxTokens;
  if (Array.isArray(tools) && tools.length) reqBody.tools = tools;
  onStage?.('upstream_request_configured', { requestedMaxTokens: requestedMaxTokens ?? null });
  const resp = await fetchUpstream(provider.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiKey}` },
    body: JSON.stringify(reqBody),
    signal,
  }, { signal, onStage });
  onStage?.('upstream_headers', { status: resp.status });
  if (!resp.ok) {
    onStage?.('upstream_http_error', { status: resp.status });
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  const toolAcc = {};
  const startedAt = performance.now();
  const deltaActivity = {
    reasoningDeltaCount: 0,
    contentDeltaCount: 0,
    toolArgumentDeltaCount: 0,
    firstReasoningMs: null,
    lastReasoningMs: null,
    firstContentMs: null,
    lastContentMs: null,
    firstToolArgumentMs: null,
    lastToolArgumentMs: null,
  };
  let lastProgressAt = 0;
  const contentHash = createHash('sha256');
  let contentBytes = 0;
  let streamComplete = false;
  let completionSeen = false;
  let finishReason = null;
  let usage = null;
  try {
    await pumpProviderSse(resp, (json) => {
      // OpenAI can send usage in a final choices:[] frame; DeepSeek attaches it to finish.
      if (json.usage) usage = safeUsage(json.usage);
      const choice = json.choices?.[0];
      if (!choice) return;
      const elapsedMs = Math.round(performance.now() - startedAt);
      // Reasoning text is neither user-facing content nor a completion signal. A throttled,
      // content-free event keeps the client informed while the provider is actively working.
      if (choice.delta?.reasoning_content) {
        deltaActivity.reasoningDeltaCount++;
        deltaActivity.firstReasoningMs ??= elapsedMs;
        deltaActivity.lastReasoningMs = elapsedMs;
        if (lastProgressAt === 0 || elapsedMs - lastProgressAt >= 5000) {
          lastProgressAt = elapsedMs || 1;
          sse(res, 'progress', { phase: 'reasoning' });
          onStage?.('upstream_reasoning_progress', {
            reasoningDeltaCount: deltaActivity.reasoningDeltaCount,
          });
        }
      }
      if (choice.delta?.content != null && typeof choice.delta.content !== 'string') {
        rejectProviderField('invalid_content_delta');
      }
      if (choice.delta?.content) {
        contentHash.update(choice.delta.content);
        contentBytes += Buffer.byteLength(choice.delta.content);
        deltaActivity.contentDeltaCount++;
        deltaActivity.firstContentMs ??= elapsedMs;
        deltaActivity.lastContentMs = elapsedMs;
        sse(res, 'chunk', { text: choice.delta.content });
      }
      const tcs = choice.delta?.tool_calls;
      if (Array.isArray(tcs)) {
        for (const tc of tcs) {
          if (!tc || typeof tc !== 'object' || (tc.function?.arguments != null && typeof tc.function.arguments !== 'string')) {
            rejectProviderField('invalid_tool_delta');
          }
          const i = tc.index ?? 0;
          toolAcc[i] = toolAcc[i] || { id: '', name: '', arguments: '' };
          if (tc.id) toolAcc[i].id = tc.id;
          if (tc.function?.name) toolAcc[i].name = tc.function.name;
          if (tc.function?.arguments) {
            toolAcc[i].arguments += tc.function.arguments;
            deltaActivity.toolArgumentDeltaCount++;
            deltaActivity.firstToolArgumentMs ??= elapsedMs;
            deltaActivity.lastToolArgumentMs = elapsedMs;
            // Large JSON arguments can stream without content or reasoning.
            // This heartbeat is content-free and never makes arguments executable.
            if (lastProgressAt === 0 || elapsedMs - lastProgressAt >= 5000) {
              lastProgressAt = elapsedMs || 1;
              sse(res, 'progress', { phase: 'tool_arguments' });
              onStage?.('upstream_tool_argument_progress', {
                toolArgumentDeltaCount: deltaActivity.toolArgumentDeltaCount,
              });
            }
          }
        }
      }
      const finish = choice.finish_reason;
      if (finish) { completionSeen = true; finishReason = finish; }
    }, onStage);
    streamComplete = completionSeen;
    // Wait for the trailing usage frame and validate EOF before releasing a complete call.
    if (finishReason === 'tool_calls') {
      Object.values(toolAcc).forEach((call, index) => onStage?.('upstream_tool_arguments_complete', toolArgumentsObservation(call, index)));
      sse(res, 'tool_calls', { tool_calls: Object.values(toolAcc), argument_proofs: Object.values(toolAcc).map(toolArgumentsObservation) });
    } else if (finishReason) {
      const proof = { textUtf8Bytes: contentBytes, textSha256: contentHash.copy().digest('hex'), contentDeltaCount: deltaActivity.contentDeltaCount };
      const accounting = { requestedMaxTokens: requestedMaxTokens ?? null, usage,
        pendingToolArguments: pendingArguments(toolAcc) };
      onStage?.('upstream_completion_observed', { stopReason: finishReason, ...proof, ...accounting });
      sse(res, 'done', { stop_reason: finishReason, text_proof: proof, completion_observation: accounting });
    }
  } finally {
    onStage?.('upstream_delta_summary', deltaActivity);
    onStage?.('upstream_capacity_summary', { requestedMaxTokens: requestedMaxTokens ?? null, usage,
      pendingToolArguments: pendingArguments(toolAcc) });
    onStage?.('upstream_text_fingerprint', { textUtf8Bytes: contentBytes, textSha256: contentHash.digest('hex'), streamComplete });
  }
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
async function streamAnthropic(provider, { model, messages, apiKey, maxTokens, tools }, res, { signal, onStage }) {
  const { system, messages: amsgs } = toAnthropicMessages(messages);
  const body = { model, messages: amsgs, stream: true, max_tokens: outputLimit(maxTokens) ?? 4096 };
  onStage?.('upstream_request_configured', { requestedMaxTokens: body.max_tokens });
  if (system) body.system = system;
  const atools = toAnthropicTools(tools);
  if (atools) body.tools = atools;

  const resp = await fetchUpstream(provider.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'x-api-key': apiKey, 'anthropic-version': '2023-06-01' },
    body: JSON.stringify(body),
    signal,
  }, { signal, onStage });
  onStage?.('upstream_headers', { status: resp.status });
  if (!resp.ok) {
    onStage?.('upstream_http_error', { status: resp.status });
    sse(res, 'error', { code: `HTTP_${resp.status}`, message: await resp.text(), retriable: resp.status >= 500 });
    return;
  }
  const toolAcc = {};
  const startedAt = performance.now();
  let lastProgressAt = -5000;
  let upstreamProgressCount = 0;
  const contentHash = createHash('sha256');
  let contentBytes = 0;
  let contentDeltaCount = 0;
  let streamComplete = false;
  let completionSeen = false;
  let finishReason = null;
  let rawUsage = {};
  const emitUpstreamProgress = () => {
    const elapsedMs = Math.round(performance.now() - startedAt);
    if (elapsedMs - lastProgressAt < 5000) return;
    lastProgressAt = elapsedMs;
    upstreamProgressCount++;
    sse(res, 'progress', { phase: 'upstream' });
    onStage?.('upstream_keepalive_progress', { upstreamProgressCount });
  };
  try { await pumpProviderSse(resp, (json) => {
    if (json.message?.usage) rawUsage = { ...rawUsage, ...json.message.usage };
    if (json.usage) rawUsage = { ...rawUsage, ...json.usage };
    if (json.type === 'ping') {
      emitUpstreamProgress();
    } else if (json.type === 'content_block_start' && json.content_block?.type === 'tool_use') {
      toolAcc[json.index] = { id: json.content_block.id, name: json.content_block.name, arguments: '' };
      emitUpstreamProgress();
    } else if (json.type === 'content_block_delta') {
      if (json.delta?.type === 'text_delta') {
        contentHash.update(json.delta.text);
        contentBytes += Buffer.byteLength(json.delta.text);
        contentDeltaCount++;
        sse(res, 'chunk', { text: json.delta.text });
      }
      else if (json.delta?.type === 'input_json_delta' && toolAcc[json.index]) {
        toolAcc[json.index].arguments += json.delta.partial_json || '';
        emitUpstreamProgress();
      } else if (json.delta?.type === 'thinking_delta' || json.delta?.type === 'signature_delta') {
        // Neither hidden thinking nor signatures are user-facing response content.
        emitUpstreamProgress();
      }
    } else if (json.type === 'message_delta' && json.delta?.stop_reason) {
      completionSeen = true;
      finishReason = json.delta.stop_reason;
    }
  }, onStage);
    streamComplete = completionSeen;
    if (finishReason === 'tool_use') {
      Object.values(toolAcc).forEach((call, index) => onStage?.('upstream_tool_arguments_complete', toolArgumentsObservation(call, index)));
      sse(res, 'tool_calls', { tool_calls: Object.values(toolAcc), argument_proofs: Object.values(toolAcc).map(toolArgumentsObservation) });
    } else if (finishReason) {
      const proof = { textUtf8Bytes: contentBytes, textSha256: contentHash.copy().digest('hex'), contentDeltaCount };
      const accounting = { requestedMaxTokens: body.max_tokens, usage: safeUsage(rawUsage), pendingToolArguments: pendingArguments(toolAcc) };
      onStage?.('upstream_completion_observed', { stopReason: finishReason, ...proof, ...accounting });
      sse(res, 'done', { stop_reason: finishReason, text_proof: proof, completion_observation: accounting });
    }
  } finally {
    onStage?.('upstream_capacity_summary', { requestedMaxTokens: body.max_tokens,
      usage: safeUsage(rawUsage), pendingToolArguments: pendingArguments(toolAcc) });
    onStage?.('upstream_text_fingerprint', { textUtf8Bytes: contentBytes, textSha256: contentHash.digest('hex'), streamComplete });
  }
}

/** 按 provider.kind 路由到对应适配器，输出归一化 SSE。 */
export async function streamProvider(provider, args, res, options = {}) {
  if (provider.kind === 'anthropic') await streamAnthropic(provider, args, res, options);
  else await streamOpenAiCompat(provider, args, res, options);
}
