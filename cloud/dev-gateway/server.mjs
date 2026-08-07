// 临时本地 dev AI 网关（PI-GW-DEV-01/02/03/06）。模块化单体：providers / auth / gate / lic / byok / db(sqlite) / kms(信封)。
//
// ⚠️ 人类授权临时偏离云 APC「云代码开发启动迁出独立仓库」约定（EXEMPTION-GW-DEV-01 扩展，云端真实化在仓迭代）。
// 正式版迁独立仓库、改 TypeScript、托管 DB + 托管 KMS（KEK 不出 KMS）+ 真实订阅门禁 + 内容不留存审计。
// 安全：BYOK 信封加密入库（kms.mjs，KEK 与密文分离）；平台/请求 key 仅内存用、不落盘、不写日志（REQ-GW-006）。

import { createServer } from 'node:http';
import { PROVIDERS, sse, streamProvider } from './providers.mjs';
import { login } from './auth.mjs';
import { checkGate } from './gate.mjs';
import { setByokKey, getByokKey, listByokProviders, deleteByokKey } from './byok.mjs';
import { recordUsage } from './usage.mjs';

const PORT = Number(process.env.GW_PORT) || 8787;

// 预设模型清单（dev；TERM-AG-010 正式版由云端运营配置经 GET /models 下发，桌面端仅消费）。
const PRESET_MODELS = [
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

const server = createServer(async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  if (req.method === 'GET' && req.url === '/health') {
    return json(res, 200, { ok: true, dev: true, providers: Object.keys(PROVIDERS) });
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
    let body;
    try {
      body = await readJson(req);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
    const name = body.provider || 'openai';
    const provider = PROVIDERS[name];
    if (!provider) return json(res, 400, { error: `unknown provider: ${name}`, providers: Object.keys(PROVIDERS) });

    // 门禁前置（X-SEQ-001）：必经裁决。令牌问题→401（客户端重登录）；额度/订阅→429（不重登录，PI-GW-DEV-05）。
    const verdict = checkGate(body.token);
    if (verdict.verdict !== 'passed') {
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

    // 密钥优先级：请求体 api_key > BYOK 代持 > 平台 env。不落盘、不记录。
    const apiKey = body.api_key || getByokKey(name) || process.env[`GW_${name.toUpperCase()}_KEY`];
    if (!apiKey) return json(res, 401, { error: `no api key for ${name}（BYOK 或 env GW_${name.toUpperCase()}_KEY）` });
    if (!body.model || !Array.isArray(body.messages)) return json(res, 400, { error: 'require { model, messages[] }' });

    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive' });
    try {
      await streamProvider(
        provider,
        { model: body.model, messages: body.messages, apiKey, maxTokens: body.max_tokens, tools: body.tools },
        res,
      );
    } catch (e) {
      sse(res, 'error', { code: 'GW_STREAM_ERR', message: String(e?.message || e), retriable: true });
    }
    res.end();
    return;
  }

  json(res, 404, { error: 'not found' });
});

server.listen(PORT, () => {
  console.log(
    `[dev-gateway] http://localhost:${PORT} — providers: ${Object.keys(PROVIDERS).join(', ')} | 模块: providers/auth/gate/lic/byok/db(sqlite)/kms(信封)`,
  );
});
