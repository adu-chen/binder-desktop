# 预览链 LibreOffice 资源目录

此目录承载**优化版** LibreOffice，供预览链运行时（`src-tauri/src/doc/libreoffice_runtime.rs`）使用。

- 部署：`./scripts/deploy_lo.sh`（或 `LO_SOURCE_APP=/path/to/LibreOffice*.app ./scripts/deploy_lo.sh`），
  将优化版 `LibreOffice.app` 放入本目录。
- 打包：`tauri.conf.json` 的 `bundle.resources`（`resources/libreoffice/**`）构建时自动打包进
  `Binder.app/Contents/Resources/libreoffice`。
- 入库：`LibreOffice.app`（~687M）经 `.gitignore` 排除，不入 git；本占位 README 提交以保证目录存在、
  使 bundle.resources glob 在未部署时仍匹配（构建不因路径为空而失败）。
- 优化：体积较 stock 减约 30%，来自自定义源码构建禁用无关组件（`--without-java` / `--disable-scripting`
  / `--disable-cups` 等，详见 `scripts/deploy_lo.sh` 头部配方）。

未部署时运行时优雅降级：`resolve_soffice_path` 找不到内置/系统 soffice → 预览返回统一错误
`DOC_LIBREOFFICE_MISSING`（不静默降级）。
