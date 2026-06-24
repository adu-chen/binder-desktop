-- Pandoc Lua 过滤器：强制保留内联样式
-- 用于 DOCX → HTML 转换时保留格式信息（对齐、颜色等）
--
-- Pandoc AST 结构说明：
-- - Header 和 Para 元素有 attr 属性，包含 attributes 表
-- - 对齐信息可能在 attributes.align 中，或者需要通过其他方式获取
-- - 样式信息应该添加到 attributes.style 中（作为字符串）
-- - 颜色信息可能在 run properties (rPr) 中，但 Pandoc 可能不会自动转换
--   我们需要通过检查 attributes 或使用其他方法获取

-- 日志函数（输出到 stderr，Pandoc 会捕获）
local function log(message)
  io.stderr:write("[Lua Filter] " .. tostring(message) .. "\n")
  io.stderr:flush()
end

-- 辅助函数：添加样式到属性，返回新的 Attr 对象
local function add_style(attr, style_str)
  if not attr then
    log("add_style: attr is nil, creating new Attr")
    attr = pandoc.Attr("", {}, {})
  end
  
  -- 确保 style_str 是字符串
  if not style_str then
    log("add_style: style_str is nil")
    return attr
  end
  if type(style_str) ~= "string" then
    log("add_style: style_str is not string, type: " .. type(style_str))
    return attr
  end
  
  -- 创建新的 attributes 表（复制现有的）
  local new_attributes = {}
  if attr.attributes then
    for k, v in pairs(attr.attributes) do
      new_attributes[k] = v
    end
  end
  
  -- 获取现有样式（可能是字符串或表）
  local existing_style = new_attributes.style or ""
  if type(existing_style) == "table" then
    existing_style = table.concat(existing_style, "; ")
  elseif type(existing_style) ~= "string" then
    existing_style = tostring(existing_style)
  end
  
  -- 提取样式属性名（如 "text-align"）
  local prop_name = style_str:match("([^:]+)")
  if prop_name then
    prop_name = prop_name:gsub("%s+", "")  -- 去除空格
    -- 检查样式是否已存在（不区分大小写）
    local existing_lower = existing_style:lower()
    local prop_lower = prop_name:lower()
    if not existing_lower:match(prop_lower) then
      if existing_style == "" then
        new_attributes.style = style_str
      else
        new_attributes.style = existing_style .. "; " .. style_str
      end
    end
  end
  
  -- 创建新的 Attr 对象
  local new_classes = {}
  if attr.classes then
    for _, cls in ipairs(attr.classes) do
      table.insert(new_classes, cls)
    end
  end
  
  return pandoc.Attr(attr.identifier or "", new_classes, new_attributes)
end

-- 辅助函数：从元素属性中提取颜色
local function extract_color(elem)
  if not elem or not elem.attr or not elem.attr.attributes then
    return nil
  end
  
  -- 检查多种可能的颜色属性
  local color = elem.attr.attributes.color or 
                elem.attr.attributes["data-color"] or
                elem.attr.attributes["foreground-color"] or
                elem.attr.attributes["w:color"]
  
  return color
end

-- 辅助函数：从元素属性中提取背景色（高亮）
local function extract_background_color(elem)
  if not elem or not elem.attr or not elem.attr.attributes then
    return nil
  end
  
  -- 检查多种可能的背景色属性
  local bg_color = elem.attr.attributes["background-color"] or
                   elem.attr.attributes["bgcolor"] or
                   elem.attr.attributes["data-bgcolor"] or
                   elem.attr.attributes["w:highlight"]
  
  return bg_color
end

-- 辅助函数：从元素属性中提取字号
local function extract_font_size(elem)
  if not elem or not elem.attr or not elem.attr.attributes then
    return nil
  end
  
  -- 检查多种可能的字号属性
  local font_size = elem.attr.attributes["font-size"] or
                    elem.attr.attributes["data-font-size"] or
                    elem.attr.attributes["w:sz"]
  
  return font_size
end

-- 辅助函数：从元素属性中提取字体
local function extract_font_family(elem)
  if not elem or not elem.attr or not elem.attr.attributes then
    return nil
  end
  
  -- 检查多种可能的字体属性
  local font_family = elem.attr.attributes["font-family"] or
                      elem.attr.attributes["data-font-family"] or
                      elem.attr.attributes["w:rFonts"]
  
  return font_family
end

-- 辅助函数：处理内联元素的格式（颜色、背景色、字号、字体）
local function process_inline_formatting(elem)
  if not elem then 
    log("process_inline_formatting: elem is nil")
    return elem 
  end
  
  if not elem.attr then
    log("process_inline_formatting: elem.attr is nil, creating new Attr")
  end
  local attr = elem.attr or pandoc.Attr("", {}, {})
  local has_changes = false
  
  -- 处理颜色
  local color = extract_color(elem)
  if color then
    -- 确保 color 是字符串
    if type(color) ~= "string" then
      color = tostring(color)
    end
    -- 转换颜色格式（如果是 RRGGBB 格式，添加 # 前缀）
    if color:match("^[0-9A-Fa-f]{6}$") then
      color = "#" .. color
    end
    attr = add_style(attr, string.format("color: %s", color))
    has_changes = true
  end
  
  -- 处理背景色（高亮）
  local bg_color = extract_background_color(elem)
  if bg_color then
    -- 确保 bg_color 是字符串
    if type(bg_color) ~= "string" then
      bg_color = tostring(bg_color)
    end
    -- 转换高亮颜色名称到 CSS 颜色值
    local highlight_colors = {
      yellow = "#FFFF00",
      green = "#00FF00",
      cyan = "#00FFFF",
      magenta = "#FF00FF",
      blue = "#0000FF",
      red = "#FF0000",
      darkBlue = "#000080",
      darkCyan = "#008080",
      darkGreen = "#008000",
      darkMagenta = "#800080",
      darkRed = "#800000",
      darkYellow = "#808000",
      lightGray = "#C0C0C0",
      darkGray = "#808080",
      black = "#000000",
      white = "#FFFFFF"
    }
    local css_color = highlight_colors[bg_color:lower()] or bg_color
    if type(css_color) == "string" and css_color:match("^[0-9A-Fa-f]{6}$") then
      css_color = "#" .. css_color
    end
    attr = add_style(attr, string.format("background-color: %s", css_color))
    has_changes = true
  end
  
  -- 处理字号
  local font_size = extract_font_size(elem)
  if font_size then
    -- 确保 font_size 是字符串或数字
    if type(font_size) ~= "string" and type(font_size) ~= "number" then
      font_size = tostring(font_size)
    end
    -- 如果是数字（半磅单位），转换为 pt
    local size_num = tonumber(font_size)
    if size_num and size_num > 0 then
      -- DOCX 使用半磅单位，需要除以 2
      local pt_size = size_num / 2
      attr = add_style(attr, string.format("font-size: %.1fpt", pt_size))
    else
      -- 已经是字符串格式（如 "12pt"），直接使用
      attr = add_style(attr, string.format("font-size: %s", tostring(font_size)))
    end
    has_changes = true
  end
  
  -- 处理字体
  local font_family = extract_font_family(elem)
  if font_family then
    -- 确保 font_family 是字符串
    if type(font_family) ~= "string" then
      font_family = tostring(font_family)
    end
    -- 如果是 XML 格式（如 w:rFonts），需要提取实际字体名
    -- 这里简化处理，直接使用
    attr = add_style(attr, string.format("font-family: %s", font_family))
    has_changes = true
  end
  
  -- 如果有更改，创建新元素
  if has_changes then
    -- 安全处理 content
    local content = {}
    if elem.content then
      if type(elem.content) == "table" then
        content = elem.content
      else
        content = {elem.content}
      end
    else
      log("process_inline_formatting: elem.content is nil, using empty table")
    end
    
    if not elem.t then
      log("process_inline_formatting: elem.t is nil, cannot create element")
      return elem
    end
    
    log("process_inline_formatting: creating " .. elem.t .. " element with " .. #content .. " content items")
    
    if elem.t == "Span" then
      return pandoc.Span(content, attr)
    elseif elem.t == "Strong" then
      return pandoc.Strong(content, attr)
    elseif elem.t == "Emphasis" then
      return pandoc.Emphasis(content, attr)
    else
      log("process_inline_formatting: unknown element type: " .. tostring(elem.t))
    end
  end
  
  return elem
end

-- 处理 Span：保留颜色、背景色、字号、字体等格式
function Span(elem)
  return process_inline_formatting(elem)
end

-- 处理 Strong（粗体）：可能包含颜色、背景色、字号、字体
function Strong(elem)
  return process_inline_formatting(elem)
end

-- 处理 Emphasis（斜体）：可能包含颜色、背景色、字号、字体
function Emphasis(elem)
  return process_inline_formatting(elem)
end

-- 辅助函数：提取段落格式（行距、首行缩进、段前段后间距）
local function extract_paragraph_formatting(attr)
  local formatting = {}
  
  if not attr then
    log("extract_paragraph_formatting: attr is nil")
    return formatting
  end
  
  if not attr.attributes then
    log("extract_paragraph_formatting: attr.attributes is nil")
    return formatting
  end
  
  -- 提取行距
  local line_spacing = attr.attributes["line-spacing"] or
                       attr.attributes["data-line-spacing"] or
                       attr.attributes["w:spacing"]
  if line_spacing then
    -- 如果是数字，可能是倍数或固定值（twips）
    local spacing_num = tonumber(line_spacing)
    if spacing_num then
      if spacing_num <= 5 then
        -- 可能是倍数（1.0, 1.5, 2.0 等）
        formatting.line_height = string.format("%.1f", spacing_num)
      else
        -- 可能是 twips，转换为 pt（1 twip = 1/20 pt）
        local pt_spacing = spacing_num / 20
        formatting.line_height = string.format("%.1fpt", pt_spacing)
      end
    else
      formatting.line_height = line_spacing
    end
  end
  
  -- 提取首行缩进
  local first_line_indent = attr.attributes["first-line-indent"] or
                            attr.attributes["data-first-line-indent"] or
                            attr.attributes["w:ind"]
  if first_line_indent then
    local indent_num = tonumber(first_line_indent)
    if indent_num then
      -- 如果是 twips，转换为 em（1 twip = 1/1440 inch ≈ 0.05em）
      local em_indent = indent_num / 1440 * 72 / 12  -- 转换为 em
      formatting.text_indent = string.format("%.2fem", em_indent)
    else
      formatting.text_indent = first_line_indent
    end
  end
  
  -- 提取段前间距
  local space_before = attr.attributes["space-before"] or
                       attr.attributes["data-space-before"]
  -- 安全访问嵌套属性
  if not space_before and attr.attributes["w:spacing"] then
    if type(attr.attributes["w:spacing"]) == "table" then
      space_before = attr.attributes["w:spacing"]["w:before"]
    end
  end
  if space_before then
    local before_num = tonumber(space_before)
    if before_num then
      local pt_before = before_num / 20  -- twips to pt
      formatting.margin_top = string.format("%.1fpt", pt_before)
    else
      formatting.margin_top = space_before
    end
  end
  
  -- 提取段后间距
  local space_after = attr.attributes["space-after"] or
                      attr.attributes["data-space-after"]
  -- 安全访问嵌套属性
  if not space_after and attr.attributes["w:spacing"] then
    if type(attr.attributes["w:spacing"]) == "table" then
      space_after = attr.attributes["w:spacing"]["w:after"]
    end
  end
  if space_after then
    local after_num = tonumber(space_after)
    if after_num then
      local pt_after = after_num / 20  -- twips to pt
      formatting.margin_bottom = string.format("%.1fpt", pt_after)
    else
      formatting.margin_bottom = space_after
    end
  end
  
  return formatting
end

-- 处理段落：保留对齐、行距、首行缩进、段前段后间距等格式
function Para(elem)
  -- 安全检查：确保 elem 存在
  if not elem then
    log("Para: elem is nil")
    return elem
  end
  
  log("Para: processing paragraph")
  
  -- 检查对齐方式（多种可能的位置）
  local align = nil
  
  -- 获取现有的 attr（如果存在）
  local attr = elem.attr or pandoc.Attr("", {}, {})
  
  -- 方法1：从 attributes.align 获取
  if attr.attributes and attr.attributes.align then
    align = attr.attributes.align
  end
  
  -- 方法2：从 class 中推断（如 "center", "text-center"）
  if not align and attr.classes then
    for _, cls in ipairs(attr.classes) do
      if cls == "center" or cls == "text-center" then
        align = "center"
        break
      elseif cls == "left" or cls == "text-left" then
        align = "left"
        break
      elseif cls == "right" or cls == "text-right" then
        align = "right"
        break
      end
    end
  end
  
  -- 如果找到对齐信息，添加到样式
  if align then
    attr = add_style(attr, string.format("text-align: %s", align))
  end
  
  -- 提取并应用段落格式（行距、首行缩进、段前段后间距）
  local para_formatting = extract_paragraph_formatting(attr)
  if para_formatting.line_height then
    attr = add_style(attr, string.format("line-height: %s", para_formatting.line_height))
  end
  if para_formatting.text_indent then
    attr = add_style(attr, string.format("text-indent: %s", para_formatting.text_indent))
  end
  if para_formatting.margin_top then
    attr = add_style(attr, string.format("margin-top: %s", para_formatting.margin_top))
  end
  if para_formatting.margin_bottom then
    attr = add_style(attr, string.format("margin-bottom: %s", para_formatting.margin_bottom))
  end
  
  -- 处理段落内容中的内联元素（颜色、背景色、字号、字体等）
  -- 安全处理：确保 content 存在且是表
  local content = {}
  if elem.content then
    if type(elem.content) == "table" then
      content = elem.content
      log("Para: content is table with " .. #content .. " items")
    else
      -- 如果不是表，创建一个包含它的表
      content = {elem.content}
      log("Para: content is not table, wrapped in table")
    end
  else
    log("Para: elem.content is nil, using empty table")
  end
  
  -- 处理内联元素
  for i, inline in ipairs(content) do
    if not inline then
      log("Para: inline[" .. i .. "] is nil, skipping")
    elseif not inline.t then
      log("Para: inline[" .. i .. "].t is nil, skipping")
    else
      -- 递归处理内联元素
      if inline.t == "Span" then
        content[i] = Span(inline)
      elseif inline.t == "Strong" then
        content[i] = Strong(inline)
      elseif inline.t == "Emphasis" then
        content[i] = Emphasis(inline)
      else
        log("Para: inline[" .. i .. "] has unknown type: " .. tostring(inline.t))
      end
    end
  end
  
  -- 创建新的 Para 元素，使用修改后的 attr 和 content
  log("Para: creating new Para with " .. #content .. " content items")
  return pandoc.Para(content, attr)
end

-- 处理标题：保留对齐
function Header(elem)
  -- 安全检查：确保 elem 存在
  if not elem then
    log("Header: elem is nil")
    return elem
  end
  
  log("Header: processing header level " .. tostring(elem.level or "unknown"))
  
  -- 检查对齐方式
  local align = nil
  
  -- 获取现有的 attr（如果存在）
  local attr = elem.attr or pandoc.Attr("", {}, {})
  
  -- 方法1：从 attributes.align 获取
  if attr.attributes and attr.attributes.align then
    align = attr.attributes.align
  end
  
  -- 方法2：从 class 中推断
  if not align and attr.classes then
    for _, cls in ipairs(attr.classes) do
      if cls == "center" or cls == "text-center" then
        align = "center"
        break
      elseif cls == "left" or cls == "text-left" then
        align = "left"
        break
      elseif cls == "right" or cls == "text-right" then
        align = "right"
        break
      end
    end
  end
  
  -- 特殊处理：根据 DOCX 样式定义，heading 1 通常是居中的
  -- 如果没有任何对齐信息，且是 h1，默认添加居中
  if not align and elem.level == 1 then
    -- 检查是否已经有 text-align 样式
    local has_text_align = false
    if attr.attributes and attr.attributes.style then
      local style = attr.attributes.style
      if type(style) == "string" then
        has_text_align = style:match("text%-align")
      elseif type(style) == "table" then
        for _, s in ipairs(style) do
          if s:match("text%-align") then
            has_text_align = true
            break
          end
        end
      end
    end
    
    if not has_text_align then
      align = "center"
    end
  end
  
  -- 如果找到对齐信息，添加到样式（add_style 现在返回新的 Attr）
  if align then
    attr = add_style(attr, string.format("text-align: %s", align))
  end
  
  -- 处理标题内容中的内联元素（颜色等）
  -- 安全处理：确保 content 存在且是表
  local content = {}
  if elem.content then
    if type(elem.content) == "table" then
      content = elem.content
    else
      -- 如果不是表，创建一个包含它的表
      content = {elem.content}
    end
  end
  
  for i, inline in ipairs(content) do
    if inline and inline.t then
      if inline.t == "Span" then
        content[i] = Span(inline)
      elseif inline.t == "Strong" then
        content[i] = Strong(inline)
      elseif inline.t == "Emphasis" then
        content[i] = Emphasis(inline)
      end
    end
  end
  
  -- 创建新的 Header 元素，使用修改后的 attr 和 content
  -- 确保 level 存在
  local level = elem.level or 1
  return pandoc.Header(level, content, attr)
end

-- 处理 Div：保留对齐
function Div(elem)
  -- 安全检查：确保 elem 存在
  if not elem then
    return elem
  end
  
  local attr = elem.attr or pandoc.Attr("", {}, {})
  
  local align = nil
  if attr.attributes and attr.attributes.align then
    align = attr.attributes.align
  end
  
  if align then
    attr = add_style(attr, string.format("text-align: %s", align))
  end
  
  -- 安全处理 content
  local content = {}
  if elem.content then
    if type(elem.content) == "table" then
      content = elem.content
    else
      content = {elem.content}
    end
  end
  
  -- 创建新的 Div 元素，使用修改后的 attr
  return pandoc.Div(content, attr)
end

