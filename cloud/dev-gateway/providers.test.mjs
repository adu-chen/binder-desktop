import { test, expect } from 'vitest';
import { streamProvider } from './providers.mjs';

function providerResponse(events) {
  const bytes = new TextEncoder().encode(events.map((event) => `data: ${JSON.stringify(event)}\n\n`).join(''));
  return new Response(new ReadableStream({
    start(controller) {
      controller.enqueue(bytes);
      controller.close();
    },
  }), { status: 200 });
}

async function runProvider(events, provider = { kind: 'openai', url: 'https://example.invalid/stream' }) {
  const originalFetch = globalThis.fetch;
  const writes = [];
  const stages = [];
  globalThis.fetch = async () => providerResponse(events);
  try {
    await streamProvider(
      provider,
      { model: 'fixed', messages: [{ role: 'user', content: 'fixed' }], apiKey: 'test-only' },
      { write: (value) => writes.push(value) },
      { onStage: (phase, detail) => stages.push({ phase, detail }) },
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
  return { output: writes.join(''), stages };
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
  expect(output).toMatch(/event: done\ndata: {"stop_reason":"stop"}/);
  expect(output).not.toMatch(/private reasoning marker|another private marker/);
  const summary = stages.find((stage) => stage.phase === 'upstream_delta_summary');
  expect(summary.detail.reasoningDeltaCount).toBe(2);
  expect(summary.detail.contentDeltaCount).toBe(1);
  expect(stages.filter((stage) => stage.phase === 'upstream_reasoning_progress')).toHaveLength(1);
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
