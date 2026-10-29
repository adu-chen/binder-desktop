import { test, expect } from 'vitest';
import { createHash } from 'node:crypto';
import { PROVIDERS, streamProvider } from './providers.mjs';

function providerResponse(events, chunkSize = Number.MAX_SAFE_INTEGER) {
  const bytes = events instanceof Uint8Array ? events : new TextEncoder().encode(
    typeof events === 'string' ? events : events.map((event) => `data: ${JSON.stringify(event)}\n\n`).join(''));
  return new Response(new ReadableStream({
    start(controller) {
      for (let start = 0; start < bytes.length; start += chunkSize) controller.enqueue(bytes.slice(start, start + chunkSize));
      controller.close();
    },
  }), { status: 200 });
}

async function runProvider(events, provider = { kind: 'openai', url: 'https://example.invalid/stream' }, options = {}) {
  const originalFetch = globalThis.fetch;
  const writes = [];
  const stages = [];
  const requests = [];
  let error;
  globalThis.fetch = async (_url, request) => {
    requests.push(JSON.parse(request.body));
    return options.response ?? providerResponse(events, options.chunkSize);
  };
  try {
    await streamProvider(
      provider,
      { model: options.model ?? 'fixed', messages: [{ role: 'user', content: 'fixed' }], apiKey: 'test-only', maxTokens: options.maxTokens },
      { write: (value) => { writes.push(value); options.onWrite?.(value); } },
      { onStage: (phase, detail) => stages.push({ phase, detail }) },
    );
  } catch (caught) {
    if (!options.captureError) throw caught;
    error = caught;
  } finally {
    globalThis.fetch = originalFetch;
  }
  return { output: writes.join(''), stages, error, requests };
}

test('reasoning produces content-free progress, never reasoning text or completion', async () => {
  const { output, stages } = await runProvider([
    { choices: [{ delta: { reasoning_content: 'private reasoning marker' } }] },
    { choices: [{ delta: { reasoning_content: 'another private marker' } }] },
    { choices: [{ delta: { content: 'visible' } }] },
    { choices: [{ delta: {}, finish_reason: 'stop' }] },
  ]);
  expect(output).toMatch(/event: progress\ndata: {"phase":"reasoning"}/);
  expect(output).toMatch(/event: chunk\ndata: {"text":"visible"}/);
  const doneFrame = output.split('\n\n').find((part) => part.startsWith('event: done'));
  expect(JSON.parse(doneFrame.slice(doneFrame.indexOf('data: ') + 6))).toEqual({
    stop_reason: 'stop', text_proof: { textUtf8Bytes: 7, contentDeltaCount: 1,
      textSha256: createHash('sha256').update('visible').digest('hex') },
    completion_observation: { requestedMaxTokens: null, usage: null, pendingToolArguments: [] },
  });
  expect(output).not.toMatch(/private reasoning marker|another private marker/);
  const summary = stages.find((stage) => stage.phase === 'upstream_delta_summary');
  expect(summary.detail.reasoningDeltaCount).toBe(2);
  expect(summary.detail.contentDeltaCount).toBe(1);
  expect(stages.filter((stage) => stage.phase === 'upstream_reasoning_progress')).toHaveLength(1);
});

test('DeepSeek Flash explicitly disables thinking while legacy selection stays unchanged', async () => {
  const flash = await runProvider([], PROVIDERS.deepseek, { model: 'deepseek-flash', maxTokens: 16384 });
  expect(flash.requests[0].thinking).toEqual({ type: 'disabled' });
  expect(flash.requests[0].max_tokens).toBe(16384);
  const legacy = await runProvider([], PROVIDERS.deepseek, { model: 'deepseek-chat', maxTokens: 8192 });
  expect(legacy.requests[0]).not.toHaveProperty('thinking');
  expect(legacy.requests[0].model).toBe('deepseek-chat');
});

test('streamed tool arguments refresh idle progress without revealing argument content', async () => {
  const privateArgument = '{"body":"private payload"}';
  const { output, stages } = await runProvider([
    { choices: [{ delta: { tool_calls: [{ index: 0, id: 'call-1', function: { name: 'binder_tool_batch', arguments: privateArgument } }] } }] },
    { choices: [{ delta: {}, finish_reason: 'tool_calls' }] },
  ]);
  expect(output).toMatch(/event: progress\ndata: {"phase":"tool_arguments"}/);
  const progress = output.split('\n\n').find((part) => part.startsWith('event: progress'));
  expect(progress).not.toContain(privateArgument);
  expect(stages.find((stage) => stage.phase === 'upstream_delta_summary').detail.toolArgumentDeltaCount).toBe(1);
  expect(output).toMatch(/event: tool_calls/);
});

test('upstream stop and length remain distinct and carry identical content-basis proof', async () => {
  for (const stopReason of ['stop', 'length', 'content_filter']) {
    const body = '<binder-turn-control>{}</binder-turn-control>\n\n| A | 未写完';
    const { output, stages } = await runProvider([
      { choices: [{ delta: { content: body.slice(0, 11) } }] },
      { choices: [{ delta: { content: body.slice(11) } }] },
      { choices: [{ delta: {}, finish_reason: stopReason }] },
    ], undefined, { chunkSize: 1 });
    const done = JSON.parse(output.split('\n\n').find(p => p.startsWith('event: done')).split('\ndata: ')[1]);
    expect(done.stop_reason).toBe(stopReason);
    expect(done.text_proof).toEqual({ textUtf8Bytes: Buffer.byteLength(body),
      textSha256: createHash('sha256').update(body).digest('hex'), contentDeltaCount: 2 });
    expect(stages.find(s => s.phase === 'upstream_completion_observed').detail.stopReason).toBe(stopReason);
  }
});

test('explicit output limits reach the real provider request and invalid limits never fetch', async () => {
  for (const provider of [{ kind: 'openai', url: 'https://example.invalid/stream' },
    { kind: 'anthropic', url: 'https://example.invalid/stream' }]) {
    const { requests, stages } = await runProvider([], provider, { maxTokens: 8192 });
    expect(requests).toHaveLength(1);
    expect(requests[0].max_tokens).toBe(8192);
    expect(stages.find(s => s.phase === 'upstream_request_configured').detail.requestedMaxTokens).toBe(8192);
    for (const maxTokens of [0, -1, 1.5, '8192', Number.MAX_SAFE_INTEGER + 1]) {
      const rejected = await runProvider([], provider, { maxTokens, captureError: true });
      expect(rejected.error.code).toBe('GW_REQUEST_INVALID');
      expect(rejected.requests).toEqual([]);
    }
  }
});

test('length retains trailing usage and incomplete arguments as metadata, never executable calls', async () => {
  const argumentsText = '{"initialContent":"' + 'PRIVATE_DOCUMENT'.repeat(5000);
  const { output, stages, requests } = await runProvider([
    { choices: [{ delta: { tool_calls: [{ index: 0, id: 'partial', function: { name: 'binder_tool_batch', arguments: argumentsText } }] } }] },
    { choices: [{ delta: {}, finish_reason: 'length' }] },
    { choices: [], usage: { prompt_tokens: 11000, completion_tokens: 8192, total_tokens: 19192,
      completion_tokens_details: { reasoning_tokens: 0 }, secret: 'PRIVATE_USAGE' } },
  ], undefined, { maxTokens: 8192, chunkSize: 1024 });
  expect(requests[0].max_tokens).toBe(8192);
  const done = JSON.parse(output.split('\n\n').find(p => p.startsWith('event: done')).split('\ndata: ')[1]);
  expect(done.stop_reason).toBe('length');
  expect(done.completion_observation).toEqual({ requestedMaxTokens: 8192,
    usage: { promptTokens: 11000, completionTokens: 8192, totalTokens: 19192, reasoningTokens: 0 },
    pendingToolArguments: [{ toolIndex: 0, argumentsUtf8Bytes: Buffer.byteLength(argumentsText),
      argumentsSha256: createHash('sha256').update(argumentsText).digest('hex'), jsonValid: false }] });
  expect(output).not.toContain('event: tool_calls');
  expect(output + JSON.stringify(stages)).not.toMatch(/PRIVATE_DOCUMENT|PRIVATE_USAGE/);
});

test('Anthropic max_tokens retains input/output usage and rejects partial tool use', async () => {
  const { output } = await runProvider([
    { type: 'message_start', message: { usage: { input_tokens: 321, output_tokens: 0 } } },
    { type: 'content_block_start', index: 0, content_block: { type: 'tool_use', id: 'partial', name: 'binder_tool_batch' } },
    { type: 'content_block_delta', index: 0, delta: { type: 'input_json_delta', partial_json: '{"calls":[' } },
    { type: 'message_delta', delta: { stop_reason: 'max_tokens' }, usage: { output_tokens: 8192 } },
  ], { kind: 'anthropic', url: 'https://example.invalid/stream' }, { maxTokens: 8192 });
  const done = JSON.parse(output.split('\n\n').find(p => p.startsWith('event: done')).split('\ndata: ')[1]);
  expect(done.stop_reason).toBe('max_tokens');
  expect(done.completion_observation.usage).toEqual({ promptTokens: 321, completionTokens: 8192, totalTokens: null, reasoningTokens: null });
  expect(done.completion_observation.pendingToolArguments[0].jsonValid).toBe(false);
  expect(output).not.toContain('event: tool_calls');
});

test('a damaged tail after tool completion cannot release an executable batch', async () => {
  const raw = 'data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"binder_tool_batch","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}\n\ndata: {BAD_TAIL}\n\n';
  const { output, error } = await runProvider(raw, undefined, { captureError: true, chunkSize: 1 });
  expect(error.code).toBe('GW_PROVIDER_PROTOCOL');
  expect(output).not.toContain('event: tool_calls');
});

test('reasoning-only stream remains incomplete despite a progress event', async () => {
  const { output, stages } = await runProvider([
    { choices: [{ delta: { reasoning_content: 'private reasoning marker' } }] },
  ]);
  expect(output).toMatch(/event: progress/);
  expect(output).not.toMatch(/event: done|event: chunk|private reasoning marker/);
  expect(stages.find((stage) => stage.phase === 'upstream_delta_summary').detail.contentDeltaCount).toBe(0);
});

test('reasoning progress leaves fragmented native tool-call completion intact', async () => {
  const { output } = await runProvider([
    { choices: [{ delta: { reasoning_content: 'private reasoning marker' } }] },
    { choices: [{ delta: { tool_calls: [{ index: 0, id: 'call-1', function: { name: 'workspace_glob', arguments: '{"pattern":' } }] } }] },
    { choices: [{ delta: { tool_calls: [{ index: 0, function: { arguments: '"*.txt"}' } }] } }] },
    { choices: [{ delta: {}, finish_reason: 'tool_calls' }] },
  ]);
  expect(output).toMatch(/event: progress/);
  expect(output).toMatch(/event: tool_calls/);
  expect(output).not.toMatch(/event: done|private reasoning marker/);
  const frame = output.split('\n\n').find((part) => part.startsWith('event: tool_calls'));
  const calls = JSON.parse(frame.slice(frame.indexOf('data: ') + 6)).tool_calls;
  expect(calls).toEqual([{ id: 'call-1', name: 'workspace_glob', arguments: '{"pattern":"*.txt"}' }]);
});

test('Anthropic keepalive and hidden thinking preserve tool-use without exposing private deltas', async () => {
  const { output, stages } = await runProvider([
    { type: 'ping' },
    { type: 'content_block_delta', index: 0, delta: { type: 'thinking_delta', thinking: 'private thought' } },
    { type: 'content_block_start', index: 1, content_block: { type: 'tool_use', id: 'call-2', name: 'workspace_glob' } },
    { type: 'content_block_delta', index: 1, delta: { type: 'input_json_delta', partial_json: '{"pattern":"*.txt"}' } },
    { type: 'message_delta', delta: { stop_reason: 'tool_use' } },
  ], { kind: 'anthropic', url: 'https://example.invalid/stream' });
  expect(output).toMatch(/event: progress\ndata: {"phase":"upstream"}/);
  expect(output).toMatch(/event: tool_calls/);
  expect(output).not.toMatch(/private thought|event: chunk|event: done/);
  const frame = output.split('\n\n').find((part) => part.startsWith('event: tool_calls'));
  const calls = JSON.parse(frame.slice(frame.indexOf('data: ') + 6)).tool_calls;
  expect(calls).toEqual([{ id: 'call-2', name: 'workspace_glob', arguments: '{"pattern":"*.txt"}' }]);
  expect(stages.some((stage) => stage.phase === 'upstream_keepalive_progress')).toBe(true);
});

test('native HTML arguments survive every byte boundary and carry the original UTF8 fingerprint', async () => {
  const args = JSON.stringify({ control: { nextAction: 'tool' }, calls: [{ toolName: 'plan_workspace_operation', params: {
    operations: [{ kind: 'create_document', targetPath: 'empty.html', resourceKind: 'file' }],
    initialContent: '<!DOCTYPE html><html><head><meta charset="utf-8"></head><body>中文🙂\n"\\</body></html>',
  } }] });
  const pieces = [args.slice(0, 31), args.slice(31, 80), args.slice(80)];
  const events = pieces.map((argumentsPart, i) => ({ choices: [{ delta: { tool_calls: [{ index: 0,
    ...(i === 0 ? { id: 'create-1' } : {}), function: { ...(i === 0 ? { name: 'binder_tool_batch' } : {}), arguments: argumentsPart },
  }] } }] }));
  events.push({ choices: [{ delta: {}, finish_reason: 'tool_calls' }] });
  for (const chunkSize of [1, 2, 7, 64, 1024]) {
    const { output, stages } = await runProvider(events, undefined, { chunkSize });
    const frame = output.split('\n\n').find((part) => part.startsWith('event: tool_calls'));
    const actual = JSON.parse(frame.slice(frame.indexOf('data: ') + 6)).tool_calls;
    expect(actual).toEqual([{ id: 'create-1', name: 'binder_tool_batch', arguments: args }]);
    const observed = stages.find((stage) => stage.phase === 'upstream_tool_arguments_complete').detail;
    const toolEnvelope = JSON.parse(frame.slice(frame.indexOf('data: ') + 6));
    expect(toolEnvelope.argument_proofs).toEqual([observed]);
    expect(observed.argumentsUtf8Bytes).toBe(Buffer.byteLength(args));
    expect(observed.argumentsSha256).toBe(createHash('sha256').update(args).digest('hex'));
    expect(JSON.stringify(observed)).not.toContain('initialContent');
    expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(true);
  }
});

test('a malformed middle frame cannot be swallowed and forwarded as truncated tool arguments', async () => {
  const before = { choices: [{ delta: { tool_calls: [{ index: 0, id: 'call', function: { name: 'binder_tool_batch', arguments: '{"control":' } }] } }] };
  const after = { choices: [{ delta: {}, finish_reason: 'tool_calls' }] };
  const raw = `data: ${JSON.stringify(before)}\n\ndata: {BROKEN_PRIVATE_FRAGMENT}\n\ndata: ${JSON.stringify(after)}\n\n`;
  for (const chunkSize of [1, 11, 4096]) {
    const { output, stages, error } = await runProvider(raw, undefined, { chunkSize, captureError: true });
    expect(error.code).toBe('GW_PROVIDER_PROTOCOL');
    expect(error.message).toBe('Provider SSE protocol rejected: invalid_json_frame');
    expect(output).not.toContain('event: tool_calls');
    expect(stages.find((stage) => stage.phase === 'upstream_frame_rejected').detail).toEqual({ frameOrdinal: 2, reasonCode: 'invalid_json_frame' });
    expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(false);
    expect(JSON.stringify(stages)).not.toContain('PRIVATE');
  }
});

test('UTF8 corruption and incomplete tails fail closed, including after a finish frame', async () => {
  const valid = 'data: {"choices":[{"delta":{"content":"ok"},"finish_reason":"stop"}]}\n\n';
  for (const [raw, reasonCode] of [
    ['data: {"choices":[]}', 'incomplete_event'],
    ['data: {"choices":[]}\n', 'incomplete_event'],
    [new Uint8Array([100, 97, 116, 97, 58, 32, 255, 10, 10]), 'invalid_utf8'],
    [valid + 'data: {BROKEN}\n\n', 'invalid_json_frame'],
  ]) {
    const { error, stages } = await runProvider(raw, undefined, { chunkSize: 1, captureError: true });
    expect(error.code).toBe('GW_PROVIDER_PROTOCOL');
    expect(stages.find((stage) => stage.phase === 'upstream_frame_rejected').detail.reasonCode).toBe(reasonCode);
    expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(false);
  }
});

test('SSE comments, empty heartbeats, CRLF multiline data and DONE preserve visible text exactly', async () => {
  const raw = ': ping\r\n\r\ndata: \r\n\r\nevent: message\r\ndata: {"choices":[\r\ndata: {"delta":{"content":"中文🙂"},"finish_reason":"stop"}]}\r\n\r\ndata: [DONE]\r\n\r\n';
  const { output, stages } = await runProvider(raw, undefined, { chunkSize: 1 });
  expect(output).toContain('data: {"text":"中文🙂"}');
  expect(output).toContain('event: done');
  expect(stages.some((stage) => stage.phase === 'upstream_frame_rejected')).toBe(false);
  expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail).toEqual({
    textUtf8Bytes: Buffer.byteLength('中文🙂'), textSha256: createHash('sha256').update('中文🙂').digest('hex'), streamComplete: true,
  });
});

test('processing callback failures propagate instead of masquerading as heartbeat frames', async () => {
  const failure = new Error('synthetic downstream write failed');
  const { error, stages } = await runProvider([{ choices: [{ delta: { content: 'visible' } }] }], undefined,
    { captureError: true, onWrite: () => { throw failure; } });
  expect(error).toBe(failure);
  expect(stages.find((stage) => stage.phase === 'upstream_frame_processing_failed').detail).toEqual({
    frameOrdinal: 1, errorName: 'Error', errorCode: null, reasonCode: null, causeName: null, causeCode: null,
  });
  expect(stages.some((stage) => stage.phase === 'upstream_frame_rejected')).toBe(false);
  expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(false);
});

test('upstream body read failure is distinguished from a malformed frame and never releases partial tools', async () => {
  const privateArgument = '{"pattern":';
  const firstFrame = new TextEncoder().encode(`data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [
    { index: 0, id: 'call-1', function: { name: 'workspace_glob', arguments: privateArgument } },
  ] } }] })}\n\n`);
  const failure = new TypeError('private network detail');
  failure.cause = Object.assign(new Error('private socket detail'), { code: 'UND_ERR_SOCKET' });
  let reads = 0;
  const response = new Response(new ReadableStream({
    pull(controller) {
      if (reads++ === 0) controller.enqueue(firstFrame);
      else controller.error(failure);
    },
  }), { status: 200 });
  const { output, stages, error } = await runProvider([], undefined, { response, captureError: true });
  expect(error).toBe(failure);
  expect(output).not.toContain('event: tool_calls');
  expect(stages.find((stage) => stage.phase === 'upstream_body_read_failed').detail).toEqual({
    frameOrdinal: 1, errorName: 'TypeError', errorCode: null, reasonCode: null, causeName: 'Error', causeCode: 'UND_ERR_SOCKET',
  });
  expect(stages.find((stage) => stage.phase === 'upstream_capacity_summary').detail.pendingToolArguments[0].jsonValid).toBe(false);
  expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(false);
  expect(JSON.stringify(stages)).not.toMatch(/private network detail|private socket detail|\{"pattern":/);
});

test('invalid provider content type is a protocol failure, not misclassified as socket failure', async () => {
  const { output, stages, error } = await runProvider([
    { choices: [{ delta: { content: { unexpected: 'private value' } } }] },
  ], undefined, { captureError: true });
  expect(error.code).toBe('GW_PROVIDER_PROTOCOL');
  expect(output).not.toContain('event: done');
  expect(stages.find((stage) => stage.phase === 'upstream_frame_processing_failed').detail).toEqual({
    frameOrdinal: 1, errorName: 'Error', errorCode: 'GW_PROVIDER_PROTOCOL', reasonCode: 'invalid_content_delta', causeName: null, causeCode: null,
  });
  expect(stages.some((stage) => stage.phase === 'upstream_body_read_failed')).toBe(false);
  expect(JSON.stringify(stages)).not.toContain('private value');
});

test('invalid tool delta is rejected before any tool call is released', async () => {
  const { output, stages, error } = await runProvider([
    { choices: [{ delta: { tool_calls: [{ index: 0, id: 'call-1', function: {
      name: 'workspace_glob', arguments: { pattern: 'private pattern' },
    } }] } }] },
    { choices: [{ delta: {}, finish_reason: 'tool_calls' }] },
  ], undefined, { captureError: true });
  expect(error.code).toBe('GW_PROVIDER_PROTOCOL');
  expect(output).not.toContain('event: tool_calls');
  expect(stages.find((stage) => stage.phase === 'upstream_frame_processing_failed').detail.reasonCode).toBe('invalid_tool_delta');
  expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.streamComplete).toBe(false);
  expect(JSON.stringify(stages)).not.toContain('private pattern');
});

test('Anthropic shares strict damaged-frame handling and safe argument fingerprints', async () => {
  const provider = { kind: 'anthropic', url: 'https://example.invalid/stream' };
  const events = [
    { type: 'ping' },
    { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: '中文' } },
    { type: 'content_block_start', index: 1, content_block: { type: 'tool_use', id: 'call', name: 'workspace_glob' } },
    { type: 'content_block_delta', index: 1, delta: { type: 'input_json_delta', partial_json: '{"pattern":"*.txt"}' } },
    { type: 'message_delta', delta: { stop_reason: 'tool_use' } },
  ];
  const { output, stages } = await runProvider(events, provider, { chunkSize: 1 });
  expect(output).toContain('event: tool_calls');
  expect(stages.find((stage) => stage.phase === 'upstream_tool_arguments_complete').detail.argumentsSha256)
    .toBe(createHash('sha256').update('{"pattern":"*.txt"}').digest('hex'));
  expect(stages.find((stage) => stage.phase === 'upstream_text_fingerprint').detail.textUtf8Bytes).toBe(6);
  const broken = await runProvider('data: {"type":"ping"}\n\ndata: {BROKEN}\n\n', provider, { captureError: true, chunkSize: 2 });
  expect(broken.error.code).toBe('GW_PROVIDER_PROTOCOL');
  expect(broken.output).not.toContain('event: tool_calls');
});
