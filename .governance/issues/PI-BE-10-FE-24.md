# Issue Trace · PI-BE-10+FE-24 音视频预览（asset 协议流式）

承接 A-PRV-I-P-01 §2（音视频→系统解码）/ A-PRV-M-T-01（只读硬边界）/ PRV-VIEW / X-FORBID-004 / X-SEQ-002。音视频文件大，走 Tauri asset 协议流式（不字节回传）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/Cargo.toml                         # tauri 增 protocol-asset feature（asset 协议必需）
  - src-tauri/tauri.conf.json                    # 开启 assetProtocol（运行时按文件授权）
  - src-tauri/src/lib.rs                          # dispatch 增 AppHandle；prv.media_src（守界 + 动态授权 asset 域）
  - src/services/prvService.ts                   # mediaSrc（prv.media_src → convertFileSrc）
  - src/components/prv/PrvPanel.tsx              # 媒体模式 <video>/<audio>（PDF 模式保留）
  - src/components/ws/FileTreeNode.tsx           # 音视频类型可点预览
  - src/styles.css                              # 媒体样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-10-FE-24.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/doc/libreoffice_runtime.rs
  - src-tauri/src/doc/pandoc_runtime.rs
confirmation: YES
scope_reason: PI-BE-10+FE-24 音视频预览走 asset 协议（流式，避免大文件字节回传）；prv.media_src 守界 + 动态授权该文件；人类已确认 asset 协议方案。
```

## Expected Behavior

- tauri.conf 开启 assetProtocol（scope 空，运行时按文件授权——最小暴露）。
- 后端 prv.media_src(relPath)：resolve_in_workspace 守界（X-FORBID-004）→ 经 AppHandle 把该**单个文件**加入
  asset 协议 scope（allow_file）→ 回绝对路径。仅授权被预览的文件，不开放整目录。
- 前端：文件树点击音视频文件（mp4/webm/mov / mp3/wav/ogg/m4a 等）→ 预览面板媒体模式，
  `convertFileSrc(abs)` → `<video controls>` / `<audio controls>` 系统解码流式播放（不进内存）；只读无编辑。
- PDF 预览模式（PI-FE-22）保留不变；媒体与 PDF 按扩展名分流。

## Validation Commands

```
cargo build
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-10-FE-24.md
```

验收：点 mp4/mp3 → 媒体播放器流式播放（不卡）；只读；PDF 预览不受影响；scoped 门禁无越界。
