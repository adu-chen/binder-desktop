# Issue Trace · PI-FE-25 图片预览（asset 协议）

承接 A-PRV-I-P-01 §2 / PRV-VIEW / X-FORBID-007 / PI-BE-10（prv.media_src 已格式无关）。修复"图片无法预览"——图片复用 asset 协议路径，纯前端。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/components/prv/PrvPanel.tsx              # image 模式 <img>
  - src/components/ws/FileTreeNode.tsx           # 图片类型可点预览
  - src/styles.css                              # 图片样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-25.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-25 图片复用已就绪的 prv.media_src（格式无关）做 asset 协议预览；纯前端；用户报告图片无法预览。
```

## Expected Behavior

- 文件树点击图片（png/jpg/jpeg/gif/webp/svg/bmp/avif）→ 预览面板 image 模式：prv.media_src（守界+授权）
  → convertFileSrc → `<img>` 只读呈现（与音视频同 asset 路径，后端无改动）。
- PDF/音视频预览不受影响；只读无编辑。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-25.md
```

验收：点图片 → 显示图片；其他预览不受影响；scoped 门禁无越界。
