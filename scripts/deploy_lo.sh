#!/usr/bin/env bash
#
# 部署「优化版」LibreOffice 到 Tauri 资源目录，供预览链运行时（doc/libreoffice_runtime.rs）使用。
#
# 为什么用优化版：预览链只需 headless 文档→PDF 转换，stock LibreOffice.app（~1GB）含大量无关组件。
# 旧项目对 LO 源码做了构建期精简，产物约 687M（较 stock 减约 30%）。优化配方为自定义源码构建的
# configure flags（来自 libreoffice/core 的 autogen.lastrun）：
#   --without-java                       去 Java 运行时（体积最大头）
#   --disable-scripting                  去 Basic/宏脚本引擎
#   --without-helppack-integration       去帮助包集成
#   --disable-xmlhelp                    去 XML 帮助
#   --with-templates=no                  不含内置模板
#   --disable-cups --disable-dbus        去打印子系统/桌面总线（headless 不需要）
#   --disable-opengl --disable-opencl    去 GPU 加速
#   --disable-pdfium                     去 PDFium
#   --disable-report-builder             去报表生成器
#   --disable-lpsolve --disable-coinmp   去线性规划求解器
#   --disable-ldap                       去 LDAP
#   --with-macosx-version-min-required=12.0
#
# ⚠️ 构建配方还包含 **Binder VCL 字体回退源码补丁**（2026-07-03 确认，pptx 字体随机匹配的根治）：
#   vcl/source/font/PhysicalFontCollection.cxx（+147：匹配失败的字体统一回退到
#     BINDER_DEFAULT_CJK_FONT / BINDER_DEFAULT_LATIN_FONT，拦截 LO 原生随缘 fallback；
#     由 BINDER_FORCE_DEFAULT_FONTS 环境变量门控，运行时经 doc/libreoffice_runtime.rs 激活）
#   vcl/quartz/salgdi.cxx（+11：CoreText 字形级 fallback 日志）
#   补丁在旧项目 LO 源码树（libreoffice/core git 工作树未提交改动）。**用 stock LO 或未打补丁的
#   源码重建都会丢失该补丁 → 字体随机匹配回归**；重建前必须移植这两处 diff。
#
# 用法：
#   ./scripts/deploy_lo.sh                          # 用默认优化版源（旧项目预构建产物）
#   LO_SOURCE_APP=/path/to/LibreOffice*.app ./scripts/deploy_lo.sh   # 指定已构建的优化版 .app
#
# 部署后由 tauri.conf.json 的 bundle.resources（resources/libreoffice/**）自动打包进
# Binder.app/Contents/Resources/libreoffice；运行时路径解析（resolve_soffice_path）即可内置识别。
# LO 二进制经 .gitignore 排除，不入 git（700M+）。

set -euo pipefail
cd "$(dirname "$0")/.."  # 仓库根

TARGET_DIR="src-tauri/resources/libreoffice"
DEST_APP="$TARGET_DIR/LibreOffice.app"

# 优化版 LO.app 源：默认指向旧项目预构建产物；可用 LO_SOURCE_APP 覆盖。
# 若需从源码重建优化版，按上方 configure flags 在 LO 源码树执行 ./autogen.sh && make。
SOURCE_APP="${LO_SOURCE_APP:-/Users/chenzhenqiang/Desktop/test/binder-project/libreoffice/core/instdir/LibreOfficeDev.app}"

echo "📦 部署优化版 LibreOffice"
echo "   源: $SOURCE_APP"
echo "   目标: $DEST_APP"

if [ ! -d "$SOURCE_APP" ]; then
  echo "❌ 源 LibreOffice .app 不存在: $SOURCE_APP"
  echo "   请设置 LO_SOURCE_APP 指向已构建的优化版 .app，或按上方配方从源码构建。"
  exit 1
fi

SRC_SOFFICE="$SOURCE_APP/Contents/MacOS/soffice"
if [ ! -f "$SRC_SOFFICE" ]; then
  echo "❌ 源中缺少 soffice 可执行文件: $SRC_SOFFICE"
  exit 1
fi

mkdir -p "$TARGET_DIR"
if [ -d "$DEST_APP" ]; then
  echo "⚠️  移除现有部署..."
  rm -rf "$DEST_APP"
fi

# 复制并规范命名为 LibreOffice.app（运行时 resolve_soffice_path 查找此名）。
echo "📂 复制中（约数百 MB，请稍候）..."
cp -R "$SOURCE_APP" "$DEST_APP"
chmod +x "$DEST_APP/Contents/MacOS/soffice"

# 校验
DEST_SOFFICE="$DEST_APP/Contents/MacOS/soffice"
if [ -x "$DEST_SOFFICE" ]; then
  echo "✅ 部署完成"
  echo "   soffice: $DEST_SOFFICE"
  echo "   体积: $(du -sh "$DEST_APP" | cut -f1)"
  echo "   版本: $("$DEST_SOFFICE" --version 2>/dev/null | head -1 || echo '（--version 探测失败，运行时仍可用）')"
else
  echo "❌ 部署校验失败：目标 soffice 不可执行"
  exit 1
fi
