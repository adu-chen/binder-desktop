// 临时本地 dev AI 网关（PI-GW-DEV-01/02/03/06）。模块化单体：providers / auth / gate / lic / byok / db(sqlite) / kms(信封)。
//
// ⚠️ 人类授权临时偏离云 APC「云代码开发启动迁出独立仓库」约定（EXEMPTION-GW-DEV-01 扩展，云端真实化在仓迭代）。
// 正式版迁独立仓库、改 TypeScript、托管 DB + 托管 KMS（KEK 不出 KMS）+ 真实订阅门禁 + 内容不留存审计。
// 安全：BYOK 信封加密入库（kms.mjs，KEK 与密文分离）；平台/请求 key 仅内存用、不落盘、不写日志（REQ-GW-006）。

import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, readFileSync, renameSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { PROVIDERS, sse, streamProvider } from './providers.mjs';
import { login } from './auth.mjs';
import { checkGate } from './gate.mjs';
import { setByokKey, getByokKey, listByokProviders, deleteByokKey } from './byok.mjs';
import { recordUsage } from './usage.mjs';

const PORT = Number(process.env.GW_PORT) || 8787;
// Frozen at process start: editing files without restarting cannot claim a new build.
const GATEWAY_BUILD = createHash('sha256')
  .update(readFileSync(new URL('./server.mjs', import.meta.url)))
  .update(readFileSync(new URL('./providers.mjs', import.meta.url))).digest('hex');

// [GW-INVOKE] 阶段日志落盘（dev 回查用）：单文件 + 单份轮转；只写身份/阶段/耗时/脱敏计量，
// 不写请求正文、响应正文、令牌或密钥（REQ-GW-006）。写失败只报告错误类别，不影响请求。
const LOG_FILE = new URL('./.data/gateway.log', import.meta.url);
const LOG_BACKUP_FILE = new URL('./.data/gateway.log.1', import.meta.url);
const LOG_MAX_BYTES = Number(process.env.GW_LOG_MAX_BYTES) || 8 * 1024 * 1024;

function appendGatewayLog(record) {
  try {
    mkdirSync(new URL('./.data/', import.meta.url), { recursive: true });
    const line = `${JSON.stringify(record)}\n`;
    const bytes = Buffer.byteLength(line);
    // 单条超过上限时不写：否则每次写入都会触发轮转，反而丢掉整份历史。
    if (bytes > LOG_MAX_BYTES) return;
    try {
      if (statSync(LOG_FILE).size + bytes > LOG_MAX_BYTES) renameSync(LOG_FILE, LOG_BACKUP_FILE);
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error;
    }
    appendFileSync(LOG_FILE, line, { mode: 0o600 });
  } catch (error) {
    console.error(`[GW-LOG] write_failed kind=${error?.code || 'unknown'}`);
  }
}

// 预设模型清单（dev；TERM-AG-010 正式版由云端运营配置经 GET /models 下发，桌面端仅消费）。
const PRESET_MODELS = [
  { provider: 'deepseek', model: 'deepseek-flash', label: 'DeepSeek Flash', source: 'preset' },
  { provider: 'deepseek', model: 'deepseek-chat', label: 'DeepSeek Chat', source: 'preset' },
  { provider: 'deepseek', model: 'deepseek-reasoner', label: 'DeepSeek Reasoner', source: 'preset' },
  { provider: 'openai', model: 'gpt-4o-mini', label: 'GPT-4o mini', source: 'preset' },
  { provider: 'anthropic', model: 'claude-sonnet-4-6', label: 'Claude Sonnet', source: 'preset' },
];

async function readJson(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  if (!chunks.length) return {};
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}
function json(res, code, obj) {
  res.writeHead(code, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(obj));
}

function traceHeader(value) {
  return typeof value === 'string' && /^[A-Za-z0-9._:-]{1,128}$/.test(value) ? value : null;
}

function invokeTrace(req) {
  const requestId = randomUUID();
  const turnId = traceHeader(req.headers['x-binder-turn-id']);
  const modelCallRound = traceHeader(req.headers['x-binder-round-index']);
  const startedAt = performance.now();
  const summary = {
    stopReason: null,
    usage: null,
    contentChunkCount: null,
    textUtf8Bytes: null,
    textSha256: null,
    completionObserved: false,
  };
  // 适配器在 HTTP 非 2xx 时不抛错而是直接回 error 事件，因此“上游失败”必须从阶段信号判定，
  // 否则终态摘要会把 401/5xx 误记成 ok。
  let failure = null;
  let finished = false;
  const emit = (phase, details = {}) => {
    if (phase === 'upstream_completion_observed' || phase === 'upstream_tool_arguments_complete') {
      summary.completionObserved = true;
    }
    if (phase === 'upstream_completion_observed') {
      summary.stopReason = details.stopReason ?? summary.stopReason;
      summary.contentChunkCount = details.contentDeltaCount ?? summary.contentChunkCount;
      summary.textUtf8Bytes = details.textUtf8Bytes ?? summary.textUtf8Bytes;
      summary.textSha256 = details.textSha256 ?? summary.textSha256;
    } else if (phase === 'upstream_capacity_summary') {
      summary.usage = details.usage ?? summary.usage;
    } else if (phase === 'upstream_http_error') {
      failure = { status: 'upstream_http_error', upstreamStatus: details.status ?? null };
    } else if (phase === 'upstream_frame_rejected') {
      failure = { status: 'upstream_protocol_error', reasonCode: details.reasonCode ?? null };
    } else if (phase === 'upstream_frame_processing_failed' || phase === 'upstream_body_read_failed') {
      failure = { status: 'upstream_stream_error' };
    }
    // 仅记录关联身份、阶段、耗时与脱敏计量；不记录请求正文、响应正文、令牌或密钥。
    const record = {
      ts: new Date().toISOString(),
      requestId,
      turnId,
      modelCallRound,
      phase,
      elapsedMs: Math.round(performance.now() - startedAt),
      ...details,
    };
    appendGatewayLog(record);
    console.info(`[GW-INVOKE] ${JSON.stringify(record)}`);
  };
  // 每个被接受的流式请求只落一条终态摘要：成功带上游 stopReason/完成证明/用量，
  // 上游失败、协议坏帧、未观测到完成信号与取消各有独立状态，便于按 turnId 回查。
  emit.finish = (status, details = {}) => {
    if (finished) return;
    finished = true;
    const resolved = status !== 'ok'
      ? status
      : failure?.status ?? (summary.completionObserved ? 'ok' : 'incomplete_stream');
    emit('invoke_completed', { ...(failure ?? {}), status: resolved, ...summary, ...details });
  };
  return emit;
}

const server = createServer(async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, X-Binder-Turn-Id, X-Binder-Round-Index');
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  if (req.method === 'GET' && req.url === '/health') {
    return json(res, 200, { ok: true, dev: true, providers: Object.keys(PROVIDERS), gatewayBuild: GATEWAY_BUILD });
  }

  // AUTH：登录签发会话令牌（dev 最小）。
  if (req.method === 'POST' && req.url === '/auth/login') {
    let body;
    try {
      body = await readJson(req);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
    return json(res, 200, login(body.email));
  }

  // CLOUD-GATE：真门禁裁决（验令牌 + 订阅；替桩恒过，X-FORBID-002 服务端裁决）。
  if (req.method === 'POST' && req.url === '/gate/check') {
    let body;
    try {
      body = await readJson(req);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
    return json(res, 200, checkGate(body.token));
  }

  // BYOK 代持（不回显密钥）。
  if (req.method === 'POST' && req.url === '/byok/keys') {
    let body;
    try {
      body = await readJson(req);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
    if (!PROVIDERS[body.provider] || !body.api_key) {
      return json(res, 400, { error: 'require { provider(openai|deepseek|anthropic), api_key }' });
    }
    setByokKey(body.provider, body.api_key);
    return json(res, 200, { ok: true, provider: body.provider });
  }

  // BYOK 已配置 provider 列表（脱敏 preview，PI-GW-02）。
  if (req.method === 'GET' && req.url === '/byok/providers') {
    return json(res, 200, { providers: listByokProviders() });
  }

  // BYOK 删除指定 provider 密钥，立即生效。
  if (req.method === 'DELETE' && req.url.startsWith('/byok/keys/')) {
    const provider = decodeURIComponent(req.url.slice('/byok/keys/'.length));
    deleteByokKey(provider);
    return json(res, 200, { ok: true, provider });
  }

  // 预设模型清单（TERM-AG-010，桌面端仅消费；订阅档位映射归正式云 GW/LIC）。
  if (req.method === 'GET' && req.url === '/models') {
    return json(res, 200, { models: PRESET_MODELS });
  }

  if (req.method === 'POST' && req.url === '/invoke/stream') {
    const trace = invokeTrace(req);
    const abortController = new AbortController();
    let disconnected = false;
    const onDisconnect = () => {
      if (res.writableEnded || disconnected) return;
      disconnected = true;
      trace('client_disconnected');
      abortController.abort();
    };
    res.once('close', onDisconnect);
    res.once('finish', () => trace('response_finished'));
    req.once('aborted', onDisconnect);
    trace('request_received');
    let body;
    try {
      body = await readJson(req);
    } catch {
      if (abortController.signal.aborted) return;
      trace('request_rejected', { reason: 'invalid_json' });
      return json(res, 400, { error: 'invalid json' });
    }
    if (abortController.signal.aborted) return;
    trace('request_body_parsed');
    const name = body.provider || 'openai';
    const provider = PROVIDERS[name];
    if (!provider) {
      trace('request_rejected', { reason: 'unknown_provider' });
      return json(res, 400, { error: `unknown provider: ${name}`, providers: Object.keys(PROVIDERS) });
    }

    // 门禁前置（X-SEQ-001）：必经裁决。令牌问题→401（客户端重登录）；额度/订阅→429（不重登录，PI-GW-DEV-05）。
    const verdict = checkGate(body.token);
    if (verdict.verdict !== 'passed') {
      trace('request_rejected', { reason: 'gate_denied' });
      const code = verdict.reauth_required ? 401 : 429;
      return json(res, code, {
        error: 'gate_denied',
        reason: verdict.reason,
        reauth_required: !!verdict.reauth_required,
        used: verdict.used,
        limit: verdict.limit,
      });
    }
    // 用量计量（PI-GW-DEV-04，REQ-GW-003）：脱敏元数据，**不含正文**（REQ-GW-006）。
    recordUsage({ userId: verdict.userId, provider: name, model: body.model });
    // 门禁通过的可关联身份（不含令牌/正文），用于把网关日志与用量行对齐。
    trace('gate_passed', { userId: verdict.userId });

    // 密钥优先级：请求体 api_key > BYOK 代持 > 平台 env。不落盘、不记录；日志只记来源类别。
    const requestKey = body.api_key || null;
    const byokKey = requestKey ? null : getByokKey(name);
    const envKey = requestKey || byokKey ? null : process.env[`GW_${name.toUpperCase()}_KEY`];
    const apiKey = requestKey || byokKey || envKey;
    const keySource = requestKey ? 'request' : byokKey ? 'byok' : envKey ? 'env' : 'missing';
    trace('api_key_resolved', { keySource });
    if (!apiKey) {
      trace('request_rejected', { reason: 'missing_api_key' });
      return json(res, 401, { error: `no api key for ${name}（BYOK 或 env GW_${name.toUpperCase()}_KEY）` });
    }
    if (!body.model || !Array.isArray(body.messages)) {
      trace('request_rejected', { reason: 'invalid_request_shape' });
      return json(res, 400, { error: 'require { model, messages[] }' });
    }
    if (body.max_tokens != null && (!Number.isSafeInteger(body.max_tokens) || body.max_tokens < 1)) {
      trace('request_rejected', { reason: 'invalid_output_limit' });
      return json(res, 400, { error: 'max_tokens must be a positive safe integer' });
    }
    // 请求形态：只记计数与上限，不记消息或工具原文。
    trace('request_accepted', {
      userId: verdict.userId,
      keySource,
      messageCount: body.messages.length,
      toolCount: Array.isArray(body.tools) ? body.tools.length : 0,
      requestedMaxTokens: body.max_tokens ?? null,
    });

    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive', 'X-Binder-Gateway-Build': GATEWAY_BUILD });
    res.flushHeaders();
    trace('response_headers_sent');
    let outcome = 'ok';
    try {
      trace('upstream_request_started', { provider: name, model: traceHeader(body.model) });
      await streamProvider(
        provider,
        { model: body.model, messages: body.messages, apiKey, maxTokens: body.max_tokens, tools: body.tools },
        res,
        { signal: abortController.signal, onStage: trace },
      );
      trace('upstream_stream_finished');
    } catch (e) {
      if (abortController.signal.aborted) {
        outcome = disconnected ? 'client_disconnected' : 'upstream_cancelled';
        trace('upstream_cancelled');
      } else {
        outcome = 'upstream_error';
        // The provider adapter records the read/processing boundary. Keep only
        // machine-safe codes here; arbitrary error messages may contain content.
        const safeCode = (value) => typeof value === 'string' && /^[A-Z][A-Z0-9_]{0,63}$/.test(value) ? value : null;
        trace('upstream_failed', {
          errorName: traceHeader(e?.name) || 'Error',
          errorCode: safeCode(e?.code),
          causeCode: safeCode(e?.cause?.code),
        });
        const protocolError = e?.code === 'GW_PROVIDER_PROTOCOL';
        sse(res, 'error', {
          code: protocolError ? 'GW_PROVIDER_PROTOCOL' : 'GW_STREAM_ERR',
          message: protocolError ? '上游模型响应格式无效，本次响应不能继续使用。' : '上游模型流异常结束，本次响应不完整。',
          retriable: true,
        });
      }
    }
    if (!res.destroyed && !res.writableEnded) res.end();
    // 终态摘要放在响应收尾之后，作为该请求在日志里的最后一条记录。
    trace.finish(outcome);
    return;
  }

  json(res, 404, { error: 'not found' });
});

server.listen(PORT, () => {
  const started = {
    ts: new Date().toISOString(),
    phase: 'gateway_started',
    port: PORT,
    build: GATEWAY_BUILD,
    providers: Object.keys(PROVIDERS),
    logFile: fileURLToPath(LOG_FILE),
    logMaxBytes: LOG_MAX_BYTES,
  };
  appendGatewayLog(started);
  console.log(
    `[dev-gateway] http://localhost:${PORT} build=${GATEWAY_BUILD} — providers: ${Object.keys(PROVIDERS).join(', ')} | 模块: providers/auth/gate/lic/byok/db(sqlite)/kms(信封)`,
  );
  console.log(`[dev-gateway] trace_log=${started.logFile} max_bytes=${LOG_MAX_BYTES}`);
});
