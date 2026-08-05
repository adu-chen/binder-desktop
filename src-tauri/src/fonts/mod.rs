//! fonts — 本地字体库只读枚举（ED-FONT-LOCAL-LIBRARY-01）。
//!
//! 编辑工具栏字体组需要"本地字体库同步"：下拉展示常用子集 + 本地可用字体。本模块提供
//! `fonts.list_fonts` 的只读枚举能力——仅返回 macOS CoreText 注册的全部字体族名，不读
//! 字体文件内容、不解析字形、不做任何写操作；不触达 workspace（X-FORBID-004 不涉及）。
//!
//! 平台：macOS 用 CoreText（CTFontManagerCopyAvailableFontFamilyNames）；其他平台降级为
//! 空列表（当前应用仅 macOS，跨平台枚举不在本 Scope，见 ED-FONT-LOCAL-LIBRARY-01）。

use crate::ipc::IpcError;

/// 枚举本地已注册字体族名（只读）。
///
/// - macOS：CTFontManagerCopyAvailableFontFamilyNames 返回全部已注册字体族（系统 + 用户安装）。
/// - 其他平台：返回空数组（不报错，前端回退常用子集）。
pub fn list_local_font_families() -> Result<Vec<String>, IpcError> {
    #[cfg(target_os = "macos")]
    {
        list_macos_font_families()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

#[cfg(target_os = "macos")]
fn list_macos_font_families() -> Result<Vec<String>, IpcError> {
    use core_text::font_manager::copy_available_font_family_names;

    // core-text 19.2 返回 CFArray<CFString>，迭代项即 CFString（IntoIterator for &CFArray）。
    let names = copy_available_font_family_names();
    let mut families = Vec::with_capacity(names.len().max(0) as usize);
    for name in names.iter() {
        // 只读读取族名，不读字体文件内容。
        families.push(name.to_string());
    }
    Ok(families)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_returns_nonempty_on_macos_or_empty_elsewhere() {
        // 契约：成功返回数组（macOS 非空；其他平台空数组），不 panic、不返回错误。
        let families = list_local_font_families().expect("字体枚举不应失败");
        if cfg!(target_os = "macos") {
            assert!(!families.is_empty(), "macOS 应有已注册字体");
        } else {
            assert!(families.is_empty());
        }
    }
}
