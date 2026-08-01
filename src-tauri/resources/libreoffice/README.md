# 预览链 LibreOffice 资源目录

此目录承载**受控内置** LibreOffice，供预览链运行时（`src-tauri/src/doc/libreoffice_runtime.rs` 与
`src-tauri/src/doc/lo_worker.rs`）使用。

- 部署：`./scripts/deploy_lo.sh`，将受控 `LibreOffice.app` 放入本目录。
- 打包：`tauri.conf.json` 的 `bundle.resources`（`resources/libreoffice/**`）构建时自动打包进
  `Binder.app/Contents/Resources/libreoffice`。
- 入库：`LibreOffice.app`（~497M）经 `.gitignore` 排除，不入 git。

## 当前状态

- **二进制**：`LibreOffice 25.8.7.3` 从源码构建（commit `30742500...`），已做后处理瘦身（移除词典
  471MB、图标主题 66MB、模板/剪贴画/宏 26MB），体积从 1.0GB 降至 497MB。
- **构建参数**：`--without-java --without-help --without-doxygen --disable-odk --disable-online-update
  --disable-mariadb-sdbc --disable-firebird-sdbc --disable-postgresql-sdbc --disable-ldap
  --disable-gpgmepp --without-webdav --disable-zxing --disable-ext-nlpsolver --disable-pdfimport
  --disable-pdfium --disable-poppler --disable-sdremote --disable-librelogo --disable-lotuswordpro
  --without-fonts`
- **VCL 插件**：仅 `libvclplug_osxlo.dylib`（Quartz），无 headless/svp 虚拟后端。
- **S1 验证**：20 次连续 docx→PDF headless 转换全部成功（exit 0，无 SIGABRT）。
- **S3 补丁**：`FindFontSubstitute` 确定性回退补丁已写入仓库但尚未编译进此二进制——需 S5 合流重建。
- `release_status` 见 `resources/tools/manifest.json`。
- **字体**：`font-plan.json`（v2）已包含 21 条策略，覆盖 100+ 别名，三结果分类（Resolved/Defaulted/Unmapped）。
  `binder-fonts/` 目录当前为空——包内字体候选待字体授权后填充。

## 运行时约束

- 只接受此目录的内置 `soffice`；不探测 `/Applications`、`PATH` 或设备安装的 LibreOffice。
- 缺失、哈希不符或版本不符时返回 `DOC_LIBREOFFICE_MISSING` / `DOC_TOOL_INVALID`。
- LoWorker 提供 120s 超时、崩溃检测（`DOC_PREVIEW_WORKER_CRASH`）、请求级隔离 profile 与过期目录清理。
