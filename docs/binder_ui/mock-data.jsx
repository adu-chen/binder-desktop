/* mock-data.jsx — all sample content for the Binder UI demo. Attaches to window. */

const WS_PATH = "~/work/binder-docs";
const WS_NAME = "binder-docs";

const FILE_TREE = [
  { id: 'docs', name: 'docs', type: 'folder', open: true, children: [
    { id: 'spec', name: 'spec.md', type: 'md', dirty: true, size: '7.2 KB', mtime: '2026-04-13' },
    { id: 'intro', name: 'intro.md', type: 'md', size: '4.8 KB', mtime: '2026-04-12' },
    { id: 'arch', name: 'architecture.md', type: 'md', size: '9.1 KB', mtime: '2026-04-11' },
    { id: 'persistence', name: 'persistence.md', type: 'md', size: '5.4 KB', mtime: '2026-04-10' },
  ]},
  { id: 'workflows', name: 'workflows', type: 'folder', open: true, children: [
    { id: 'release', name: 'release-checklist.md', type: 'md', size: '2.1 KB', mtime: '2026-04-09' },
    { id: 'review', name: 'doc-review.md', type: 'md', size: '3.0 KB', mtime: '2026-04-08' },
  ]},
  { id: 'assets', name: 'assets', type: 'folder', open: false, children: [
    { id: 'logo', name: 'binder-logo.svg', type: 'img', size: '1.4 KB', mtime: '2026-03-30' },
    { id: 'chart', name: 'tauri-commands.xlsx', type: 'xlsx', size: '38 KB', mtime: '2026-04-02' },
    { id: 'deck', name: 'roadmap-2026.pptx', type: 'pptx', size: '512 KB', mtime: '2026-04-05' },
  ]},
  { id: 'notes', name: 'notes.txt', type: 'txt', size: '1.2 KB', mtime: '2026-04-13' },
  { id: 'readme', name: 'README.md', type: 'md', size: '3.6 KB', mtime: '2026-04-01' },
  { id: 'binder-dir', name: '.binder', type: 'folder', open: false, system: true, children: [
    { id: 'wsdb', name: 'workspace.db', type: 'db', system: true, size: '128 KB' },
    { id: 'srdb', name: 'search.db', type: 'db', system: true, size: '64 KB' },
  ]},
];

const FILE_ICONS = {
  md: 'M↓', txt: 'TXT', html: '<>', docx: 'DOC', xlsx: 'XLS', pptx: 'PPT',
  pdf: 'PDF', img: 'IMG', db: 'DB',
};

const OPEN_TABS = [
  { id: 'spec', name: 'spec.md', dirty: true },
  { id: 'intro', name: 'intro.md', dirty: false },
  { id: 'persistence', name: 'persistence.md', dirty: false },
];

const CHAT_TABS = [
  { id: 'c1', name: 'Diff 审阅链路', mode: 'agent', active: true },
  { id: 'c2', name: '工具矩阵梳理', mode: 'agent' },
  { id: 'c3', name: '总览初稿', mode: 'chat' },
];

const MESSAGES_DIFF = [
  { kind: 'user', who: 'YOU', when: '14:32',
    body: [
      '把第 4.7 节 "聊天模式与工具调用" 里关于 chat 模式的描述收紧一点，',
      '强调它不启用工具，并把工具矩阵改成按类别分组。'
    ].join('\n'),
    refs: [
      { type: 'file', text: 'spec.md · §4.7' },
      { type: 'kn', text: 'KN · tauri-commands' },
    ],
  },
  { kind: 'assistant', who: 'BINDER', when: '14:33',
    body: '已读取 spec.md 的 §4.7 与工具注册表。我把工具按 read / write / structural / editor / dependency 五类重排，并在 chat 模式段落里去掉了工具相关表述。',
    tools: [
      { name: 'read_file', status: 'done', args: { path: 'src/spec.md', range: '§4.7' } },
      { name: 'list_files', status: 'done', args: { path: 'src-tauri/src/commands', filter: '*.rs' } },
      { name: 'edit_current_editor_document', status: 'pending', args: { path: 'spec.md', changes: '2 hunks · §4.7' } },
    ],
    diff: {
      file: 'spec.md',
      scope: '§4.7 · 聊天模式与工具调用',
      hunks: 2, added: 11, removed: 6,
    },
  },
];

const MESSAGES_TIMELINE = [
  { kind: 'user', who: 'YOU', when: '11:08',
    body: '上一次 4 小时前对 architecture.md 的批量重构没必要保留，回到那之前的状态预览一下。',
    refs: [ { type: 'file', text: 'architecture.md' }, { type: 'mem', text: 'MEM · 习惯 ⌘Z 撤销过深' } ],
  },
  { kind: 'assistant', who: 'BINDER', when: '11:08',
    body: '当前 workspace 有 1 个文件存在脏改（spec.md），恢复前会先要求你处理。预览已挂载在左侧时间轴节点 #1284。',
    tools: [
      { name: 'list_files', status: 'done', args: { path: 'docs/' } },
      { name: 'timeline_preview', status: 'done', args: { node: '#1284', files: '4' } },
    ],
  },
];

const MESSAGES_WELCOME = [
  { kind: 'user', who: 'YOU', when: 'now',
    body: '帮我用一段话解释 Binder 的 diff 审阅机制，再给一个最小流程图。',
  },
  { kind: 'assistant', who: 'BINDER', when: 'now',
    body: 'AI 修改不会直接落盘——所有改动先写入 workspace.db 的 pending_diffs，叠加在编辑器上以行级 diff 呈现；用户在底部待确认面板里选择 接受 / 拒绝，确认后才真正写入磁盘并刷新文件缓存。',
  },
];

/* Sample documents (TipTap-mock as React trees) — keyed by file id */
const DOC_SPEC = {
  meta: { code: 'PROD-C-R-03', type: '项目总览 / 当前实现总览', updated: '2026-04-13', version: '0.1.0' },
  title: 'Binder · 项目介绍',
};

/* Memory entries */
const MEMORIES = [
  { id: 'm1', title: '当前正在编辑的文件必须走 edit_current_editor_document', layer: 'WS', updated: '2 天前', tags: ['workflow','edit'], body: '非当前文件才走 update_file 或 workspace 级 diff 链。' },
  { id: 'm2', title: '只支持 OpenAI / DeepSeek 两个 provider', layer: 'WS', updated: '3 天前', tags: ['ai','provider'], body: '不要在 UI 上承诺 Anthropic、Gemini 等。' },
  { id: 'm3', title: '我习惯用 ⌘K 做局部修改而不是续写', layer: 'USER', updated: '上周', tags: ['习惯'], body: 'Cmd/Ctrl+J 续写、Cmd/Ctrl+K 局部修改。' },
  { id: 'm4', title: 'Excel/PPT 当前只能预览不能编辑', layer: 'WS', updated: '上周', tags: ['boundary'], body: '在 PRD 中写"已实现"会被打回。' },
  { id: 'm5', title: '时间轴前端列表上限 50 条', layer: 'WS', updated: '上周', tags: ['ui'], body: '超过 50 条要做分页；轮询刷新。' },
];

/* Knowledge entries */
const KNOWLEDGE = [
  { id: 'k1', title: 'Tauri Command 注册表（截至 0.1.0）', tag: '已验证', tagKind: 'forest', updated: '2026-04-13', body: 'main.rs 当前注册 104 个 command；覆盖文件、AI、搜索、记忆、知识等域。', refs: 12 },
  { id: 'k2', title: 'workspace.db Schema v8', tag: '已验证', tagKind: 'forest', updated: '2026-04-12', body: 'file_cache · pending_diffs · file_dependencies · agent_tasks · agent_artifacts · 记忆 · timeline_*', refs: 8 },
  { id: 'k3', title: 'Pandoc 与 LibreOffice 的依赖边界', tag: '需复核', tagKind: 'gold', updated: '2026-04-09', body: '文档转换与预览依赖外部工具；缺失时影响功能可用性。', refs: 3 },
  { id: 'k4', title: 'TipTap 3 + ProseMirror 扩展清单', tag: '已验证', tagKind: 'forest', updated: '2026-04-08', body: 'BlockIdExtension · DiffDecorationExtension · CopyReferenceExtension · PageTopCaretExtension · FontSize 等。', refs: 5 },
  { id: 'k5', title: 'Chat Build 主链定义', tag: '已验证', tagKind: 'forest', updated: '2026-04-06', body: '从 chat 模式发起 → outline 确认 → 步骤执行 → 中断/完成总结 → 打开产物。', refs: 7 },
];

/* Templates */
const TEMPLATES = [
  { id: 't1', title: '产品总览文档', code: 'PROD-C-R', status: '已启用', tagKind: 'forest', steps: 6, body: '项目概述 · 规模 · 界面 · 能力 · 边界 · 技术架构 · 持久化 · 总结。' },
  { id: 't2', title: '周会纪要', code: 'OPS-M-R', status: '已启用', tagKind: 'forest', steps: 4, body: '议题列表 · 讨论摘要 · 决议 · Action items。' },
  { id: 't3', title: '功能 PRD', code: 'FEAT-P', status: '草稿', tagKind: 'gold', steps: 8, body: '问题 · 目标 · 用户 · 场景 · 方案 · 边界 · 度量 · 风险。' },
  { id: 't4', title: '事故复盘', code: 'INC-PM', status: '已禁用', tagKind: 'rust', steps: 5, body: '时间线 · 影响面 · 直接原因 · 根因 · 改进项。' },
];

/* Timeline nodes */
const TIMELINE = [
  { id: '1287', time: '14:33', date: '今天', title: 'AI 编辑 · spec.md', desc: '§4.7 工具矩阵重排（待确认）', kind: 'diff', pending: true },
  { id: '1286', time: '14:28', date: '今天', title: '保存 · spec.md', desc: '手动保存', kind: 'save' },
  { id: '1285', time: '13:50', date: '今天', title: 'AI 编辑 · spec.md', desc: '§2.3 技术分布修正（已接受）', kind: 'diff' },
  { id: '1284', time: '11:08', date: '今天', title: '批量重构 · architecture.md', desc: '4 处段落重写', kind: 'diff', highlight: true },
  { id: '1283', time: '10:42', date: '今天', title: '新建 · workflows/doc-review.md', desc: '从模板 OPS-M-R 创建', kind: 'create' },
  { id: '1282', time: '09:14', date: '今天', title: '重命名 · old.md → spec.md', desc: '资源结构变更', kind: 'rename' },
  { id: '1281', time: '18:22', date: '昨天', title: 'AI 编辑 · intro.md', desc: '首段重写（已接受）', kind: 'diff' },
  { id: '1280', time: '17:05', date: '昨天', title: '导入 · roadmap-2026.pptx', desc: '从 Downloads 拖入', kind: 'import' },
];

const RECENT_WORKSPACES = [
  { name: 'binder-docs', path: '~/work/binder-docs', files: 247, when: '今天 14:32', current: true },
  { name: 'product-prd-q2', path: '~/work/product-prd-q2', files: 38, when: '昨天 22:18' },
  { name: 'research-llm-eval', path: '~/research/llm-eval', files: 92, when: '4 月 11 日' },
  { name: 'personal-journal', path: '~/notes/journal', files: 412, when: '4 月 8 日' },
];

/* Pending diffs index */
const PENDING_DIFFS = [
  { id: 'd1', file: 'spec.md', scope: '§4.7', added: 11, removed: 6, current: true },
  { id: 'd2', file: 'spec.md', scope: '§3.4', added: 3, removed: 1, current: true },
  { id: 'd3', file: 'README.md', scope: 'head', added: 4, removed: 2 },
];

/* ────────────────────────────────────────────────────────────
   Object states catalog — used in showcase artboards
   ──────────────────────────────────────────────────────────── */

const DIFF_STATES = [
  { id: 'pending',     code: '01', label: '待决策',     en: 'PENDING',     hint: 'AI 已生成变更，等待用户决策' },
  { id: 'validating',  code: '02', label: '校验中',     en: 'VALIDATING',  hint: '正在校验原文是否仍匹配' },
  { id: 'applying',    code: '03', label: '应用中',     en: 'APPLYING',    hint: '校验通过，正在写入文档' },
  { id: 'accepted',    code: '04', label: '已接受',     en: 'ACCEPTED',    hint: '内容已写入，不可再操作' },
  { id: 'rejected',    code: '05', label: '已拒绝',     en: 'REJECTED',    hint: '用户主动拒绝（语义不同于失效）' },
  { id: 'expired',     code: '06', label: '已失效',     en: 'EXPIRED',     hint: '情境已变化，可重新生成' },
  { id: 'conflict',    code: '07', label: '存在冲突',   en: 'CONFLICT',    hint: '与另一处 Diff 范围重叠' },
  { id: 'error',       code: '08', label: '执行错误',   en: 'ERROR',       hint: '执行过程中发生错误，可重试' },
];

const EXPIRED_REASONS = [
  { key: 'document_revision_advanced', label: '文档已被手动编辑',     en: 'DOC REVISION ADVANCED' },
  { key: 'block_resolve_failed',       label: '原始块已不可定位',     en: 'BLOCK RESOLVE FAILED' },
  { key: 'overlapping_range',          label: '与已接受 Diff 范围重叠', en: 'OVERLAPPING RANGE' },
  { key: 'external_file_changed',      label: '文件在磁盘上被外部修改', en: 'EXTERNAL FILE CHANGED' },
  { key: 'apply_replace_failed',       label: '执行替换失败（多次重试后）', en: 'APPLY REPLACE FAILED' },
];

const TOOL_STATES = [
  { id: 'pending',                code: '01', label: '已排队',      en: 'PENDING' },
  { id: 'executing',              code: '02', label: '执行中',      en: 'EXECUTING' },
  { id: 'awaiting_confirmation',  code: '03', label: '等待确认',    en: 'AWAITING CONFIRMATION' },
  { id: 'done',                   code: '04', label: '已完成',      en: 'DONE' },
  { id: 'failed',                 code: '05', label: '失败',        en: 'FAILED' },
  { id: 'skipped',                code: '06', label: '已跳过',      en: 'SKIPPED' },
];

const EDITOR_STATES = [
  { id: 'empty',            label: '空',              en: 'EMPTY' },
  { id: 'loading',          label: '加载中',          en: 'LOADING' },
  { id: 'ready',            label: '就绪',            en: 'READY' },
  { id: 'editing',          label: '编辑中',          en: 'EDITING' },
  { id: 'saving',           label: '保存中',          en: 'SAVING' },
  { id: 'saved',            label: '已保存',          en: 'SAVED' },
  { id: 'dirty',            label: '未保存',          en: 'DIRTY' },
  { id: 'reviewing',        label: '审阅中',          en: 'REVIEWING' },
  { id: 'external_changed', label: '外部已改',        en: 'EXTERNAL CHANGED' },
  { id: 'readonly_error',   label: '不可编辑',        en: 'READONLY / ERROR' },
];

const TAB_STATES = [
  { id: 'normal',           label: '默认',            badge: null,        dirty: false },
  { id: 'dirty',            label: '未保存',          badge: null,        dirty: true },
  { id: 'saving',           label: '保存中',          badge: '· · ·',     dirty: false, saving: true },
  { id: 'saved',            label: '已保存',          badge: '✓',         dirty: false },
  { id: 'pending_diff',     label: 'Pending Diff',    badge: '3',         dirty: true,  pending: true },
  { id: 'external_changed', label: '外部已改',        badge: '!',         dirty: false, warn: true },
];

const FILE_NODE_STATES = [
  { id: 'normal',           label: '默认',            badge: null },
  { id: 'open',             label: '已打开',          badge: null,  active: true },
  { id: 'dirty_open',       label: '已打开 · 未保存', badge: '●',   active: true, dirty: true },
  { id: 'pending_diff',     label: 'Pending Diff',    badge: '3',   active: true, pending: true },
  { id: 'external_changed', label: '外部已改',        badge: '!',   warn: true },
  { id: 'not_supported',    label: '不可编辑',        badge: null,  disabled: true },
];

/* ──────────────────────────────────────────────────────────
   Flow content — step frames for chains A, B, C, D, F, G, H
   ────────────────────────────────────────────────────────── */

const FLOW_A = [
  { id: 'a1', title: '① 用户在 Chat 发起请求',
    user: '帮我把 §4.7 的工具矩阵按类别分组，并强调 chat 模式不启用工具。',
    state: 'idle',
  },
  { id: 'a2', title: '② AI 思考',  state: 'thinking',  hint: '消息区显示思考动效' },
  { id: 'a3', title: '③ 工具流式执行', state: 'tool_calling', hint: 'read_file → list_files → edit_current_editor_document' },
  { id: 'a4', title: '④ 编辑器 Green Review 出现', state: 'reviewing', hint: '右侧 Diff Card(pending) 同步出现' },
  { id: 'a5', title: '⑤ 用户接受',  state: 'accepted',  hint: 'Green Review 消失，内容写入' },
];

const FLOW_B = [
  { id: 'b1', title: '① 3 条 Diff 同时存在', state: 'pending',     hint: 'DiffAllActionsBar 浮现' },
  { id: 'b2', title: '② 全部接受 · 倒序执行', state: 'applying',  hint: '从靠后的 Diff 开始，避免位置偏移' },
  { id: 'b3', title: '③ 全部完成 · 1 条 expired', state: 'mixed', hint: '某条因手动编辑被跳过' },
];

const FLOW_C = [
  { id: 'c1', title: '① 多条 Diff 生成完毕', state: 'pending' },
  { id: 'c2', title: '② 用户手动编辑被覆盖段落', state: 'document_revision_advanced' },
  { id: 'c3', title: '③ Diff 自动 expired，引导重新生成', state: 'expired' },
];

const FLOW_D = [
  { id: 'd1', title: '① 外部程序保存了同一文件', state: 'detected' },
  { id: 'd2', title: '② 顶部 Banner 询问', state: 'banner' },
  { id: 'd3', title: '③ 选择加载外部更改 · 全部 Diff 失效', state: 'reloaded' },
];

const FLOW_F = [
  { id: 'f1', title: '① 选中文本 + ⌘K', state: 'input' },
  { id: 'f2', title: '② AI 生成中', state: 'generating' },
  { id: 'f3', title: '③ 建议预览 · 对比', state: 'preview' },
  { id: 'f4', title: '④ Apply 后选区已替换', state: 'applied' },
];

const FLOW_G = [
  { id: 'g1', title: '① 用户点关闭，但有 5 条 Pending Diff', state: 'dialog' },
];

const FLOW_H = [
  { id: 'h1', title: '① AI 请求执行 delete_file', state: 'awaiting' },
  { id: 'h2', title: '② 用户授权 · 工具继续', state: 'continued' },
];

window.MOCK = {
  WS_PATH, WS_NAME,
  FILE_TREE, FILE_ICONS, OPEN_TABS, CHAT_TABS,
  MESSAGES_DIFF, MESSAGES_TIMELINE, MESSAGES_WELCOME,
  DOC_SPEC,
  MEMORIES, KNOWLEDGE, TEMPLATES, TIMELINE,
  RECENT_WORKSPACES, PENDING_DIFFS,
  DIFF_STATES, EXPIRED_REASONS, TOOL_STATES,
  EDITOR_STATES, TAB_STATES, FILE_NODE_STATES,
  FLOW_A, FLOW_B, FLOW_C, FLOW_D, FLOW_F, FLOW_G, FLOW_H,
};
