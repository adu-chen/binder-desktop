# Binder 包内转换工具

此目录是 DOC 转换运行时的唯一工具来源。`manifest.json` 锁定每个二进制的版本、架构、哈希、来源与许可；
`npm run verify:bundled-tools` 在前端构建和 Tauri 打包前校验它们。

- Pandoc 3.10 arm64：从 Pandoc 官方 GitHub Release 获取；使用
  `PANDOC_ARCHIVE=/absolute/path/pandoc-3.10-arm64-macOS.zip bash scripts/prepare_bundled_tools.sh` 准备。
- LibreOffice：使用经 Binder 补丁、签名的优化版部署到 `resources/libreoffice/LibreOffice.app`；当前仍受
  `BLOCK-REL-LO-001` 发布阻断，系统 LibreOffice 不得作为替代。

二进制不纳入 Git 普通对象；发布/CI 必须先准备受 manifest 校验的资产，再执行构建。
