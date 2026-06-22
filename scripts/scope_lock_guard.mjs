#!/usr/bin/env node
// PreToolUse 钩子：编辑时刻强制 ADP Scope Lock。
// 仅当项目根存在 .governance/active_scope.json（活跃 Scope Lock）时生效：
//   - confirmation 必须为 YES，否则阻断任何文件编辑（ADP §3.1：PENDING 时不得修改文件）
//   - 目标文件必须落在 allowed_files 内、且不在 forbidden_files 内，否则阻断（SCOPE_VIOLATION）
// 无活跃 Scope Lock（文件不存在）时一律放行，不干扰 DOC_ONLY / CODE_CHANGE_MINOR / 未开始的任务。
//
// 协议：读取 stdin 的 PreToolUse JSON；放行 exit 0；阻断 exit 2 并在 stderr 给出原因。
// 失败开放（fail-open）：自身解析或 IO 异常时放行，避免钩子故障锁死会话。

import { readFileSync, existsSync } from 'fs';
import { join, isAbsolute, relative } from 'path';

const EDIT_TOOLS = new Set(['Edit', 'Write', 'MultiEdit']);
const SCOPE_FILE_REL = '.governance/active_scope.json';

function allow() { process.exit(0); }
function block(reason) { process.stderr.write(`[scope_lock_guard] ${reason}\n`); process.exit(2); }

let raw = '';
try { raw = readFileSync(0, 'utf8'); } catch { allow(); }

let payload;
try { payload = JSON.parse(raw); } catch { allow(); }

const toolName = payload?.tool_name;
if (!EDIT_TOOLS.has(toolName)) allow();

const cwd = payload?.cwd || process.cwd();
const scopePath = join(cwd, SCOPE_FILE_REL);
if (!existsSync(scopePath)) allow(); // 无活跃 Scope Lock

let scope;
try { scope = JSON.parse(readFileSync(scopePath, 'utf8')); } catch { allow(); }

const filePath = payload?.tool_input?.file_path;
if (!filePath) allow();

// 目标文件归一化为相对项目根的路径
let rel = isAbsolute(filePath) ? relative(cwd, filePath) : filePath;
rel = rel.split('\\').join('/');

// Scope Lock 文件自身始终可写，以便建立/更新锁
if (rel === SCOPE_FILE_REL || rel.startsWith('.governance/')) allow();

function stripComment(s) { const h = s.indexOf('#'); return (h === -1 ? s : s.slice(0, h)).trim(); }
function matchesPattern(file, pattern) {
  const p = stripComment(pattern);
  if (!p) return false;
  if (p.endsWith('/**')) { const prefix = p.slice(0, -3); return file === prefix || file.startsWith(`${prefix}/`); }
  return file === p;
}
function matchesAny(file, patterns) { return (patterns || []).some(pat => matchesPattern(file, pat)); }

const confirmation = String(scope.confirmation || '').toUpperCase();
if (confirmation !== 'YES') {
  block(`Scope Lock confirmation 当前为 "${scope.confirmation ?? '未设置'}"，人类确认 YES 前不得修改文件（ADP §3.1）。目标：${rel}`);
}

if (matchesAny(rel, scope.forbidden_files)) {
  block(`文件 ${rel} 命中 Scope Lock forbidden_files，禁止修改（SC-01 SCOPE_VIOLATION）。`);
}

if (!matchesAny(rel, scope.allowed_files)) {
  block(`文件 ${rel} 不在 Scope Lock allowed_files 范围内（SC-01 SCOPE_VIOLATION）。如需扩展范围，按 ADP §3.3 等待人类确认。`);
}

allow();
