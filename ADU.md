# ADU — AI 开发通用治理标准

---

## 前言

本文档是所有治理规则的唯一来源。
以下五份文件构成完整治理控制体系。

**ADU.md**
所有治理规则的唯一来源。
包含：代码头规范、规则体系、文档体系、
      triage 协议、Issue Trace 协议。

**ADP.md**
AI 开发过程运行时协议。
消费本文档中的静态定义，控制单次开发任务的状态、Gate、
Scope Lock、Stop Condition 和运行时报告。

**CLAUDE.md**
AI 执行协议。
控制 AI 的会话入口、文档读取顺序和绝对禁止行为。

**APC**
项目级 AI 开发宪章（每个项目一份）。
路径由 governance.config.json 的 apc_path 字段声明。
定义项目身份、AI 执行边界、文档体系声明、audit 阻断策略。

**ADUS**
项目级派生检索索引（每个项目一份，机器生成）。
路径由 governance.config.json 的 adus_path 字段声明。
所有内容均由脚本从设计文档标注块和源代码 @GOV 标注自动生成。
禁止人工编辑。

**关于规则生成的原生能力：**
ADU.md 定义了规则和链路的生成方法论。
规则和链路可以在任何时候根据本文档的方法论生成，
不依赖 ADUS.md 已有内容作为前提条件。
ADUS.md 中已有的规则和链路，用于后续治理时的重复性校验，
不是规则生成的门槛。

---

## 名词定义

本文档中以下名词含义固定，不得混用：

| 名词 | 定义 |
|------|------|
| 规则ID | BR-[模块]-[域]-[序号] 格式的规则标识符，在设计文档中定义，由 AI 生成 |
| 代码块ID | [TYPE]-[MODULE]-[CHAIN]-[SEQ] 格式的唯一标识符，在代码实现阶段由 AI 生成 |
| 映射ID | [规则ID]-[代码块ID] 格式的复合标识符，表达一条规则约束一个代码块的有向连接 |
| codes字段 | @GOV 中存储映射ID的字段，一个代码块可有多个映射ID |
| 链路 | 一条完整业务执行流程，以 chain_id 标识 |
| 链路ID | chain_id，链路的唯一标识符，格式自定义，示例：DIFF-ACCEPT |
| 规则域 | 规则ID中的分类标签，表示规则所属的功能领域，示例：STATE、DATA |
| 跨模块约束 | X-[类别]-[序号] 格式的约束标识符，约束多个模块的共同行为 |
| 治理控制文档 | ADU.md、ADP.md、CLAUDE.md、APC、ADUS 五份文件的统称；其中 APC 和 ADUS 为项目级，路径由 governance.config.json 声明 |
| triage | 对 rules:[] 块的分类处理过程，产物是 triage 报告 |
| @GOV | 写在代码块注释里的结构化标注，包含五个必填字段 |
| 设计文档 | L1-L3 层级的项目文档，是规则的一手来源 |
| 规则注册表 | ADUS.md 中存储所有规则条目元信息的表格，由脚本自动生成 |
| 阶段一 | 文档治理阶段，实现开始前，产出规则和链路定义 |
| 阶段二 | 代码治理阶段，实现过程中及之后，产出 @GOV 标注和 triage 报告 |

---

## 第零节：项目接入引导

### 0.1 接入前提

使用本标准的项目需要满足以下前提：
  项目使用版本控制（Git 或同类工具）
  项目有明确的模块划分
  项目团队接受文档驱动开发的工作方式

### 0.2 接入步骤

**接入前提（在开始任何步骤之前确认）：**

  docs 文件格式：所有设计文档必须使用 ADU 标注块格式
    （<!-- RULE/CHAIN/MODULE/CONSTRAINT/TERM -->），不得使用 ## heading 替代
    其中 TERM 为可选，仅在需要锚定核心业务概念时使用

  @GOV 格式：所有代码标注必须使用块注释格式（/** @GOV ... */）

  脚本分离：package.json 中 governance:audit 和 governance:generate
    必须指向不同脚本

  路径配置：项目根目录必须存在 governance.config.json，
    包含 project_root、adus_path 和 apc_path 字段。
    generate_adus.mjs 从 governance.config.json 读取 adus_path 作为输出路径。
    adus_path 指向项目 docs/ 体系内的路径，不与治理控制文档平级。

  docs 文档头：每份设计文档必须包含标准文档头（doc_id/doc_status 等字段）

  测试覆盖注释：测试文件必须包含 // covers: BR-[模块]-[域]-[序号] 注释

**步骤一：初始化治理控制文档**

将以下文件放入项目根目录：
  ADU.md                  本文件，不修改内容
  ADP.md                  AI 开发过程运行时协议，消费 ADU 静态定义
  CLAUDE.md               AI 执行协议，不修改内容
  governance.config.json  路径配置，bootstrap 完成后由 AI 写入 apc_path / adus_path

注：ADUS.md 不放根目录，由 governance:generate 根据 governance.config.json
中的 adus_path 生成到项目 docs/ 体系内。

**步骤二：在设计文档中定义项目元数据**

在第一份设计文档中写入以下标注块：

1. 定义模块编码（每个功能模块一个）：

   ```
   <!-- MODULE
   module_code: [AI 自动推断，见 §3.0]
   module_name: [模块名称]
   description: [职责描述]
   -->
   ```

   AI 根据业务语义自动推断 module_code，
   无冲突直接写入，有歧义时输出 MODULE_AMBIGUOUS 等待裁决。

2. 若需要新增规则域（超出六个内置域）：

   ```
   <!-- DOMAIN
   domain_code: [域编码]
   description: [含义描述]
   -->
   ```

3. 链路通过在设计文档中写入 `<!-- CHAIN -->` 标注块来定义。

4. 若需要锚定核心业务概念（跨 session 容易漂移、代码中有实体承载），在设计文档开头
   `## 0. Terminology Registry` 区域写入 `<!-- TERM -->` 标注块（见 §3.2 TERM 部分）。
   TERM 为可选，不是每份文档都需要。

5. 运行脚本生成索引：
   `npm run governance:generate`

**步骤三：确认 governance.config.json 配置**

bootstrap 流程完成后，governance.config.json 会自动写入以下字段：
  apc_path      APC 文件路径（相对于 governance.config.json 所在目录）
  adus_path     ADUS 输出路径（相对于 governance.config.json 所在目录）
  test_governance_path  项目测试治理注册表路径；由项目初始化或专项治理明确写入

bootstrap 前请手动确认 governance.config.json 中 project_root 已正确填写：
  project_root  项目根目录相对路径，脚本以此为基准扫描 docs/src/tests 子目录
                示例："."（当前目录即项目根）或 "example"（子目录项目）

apc_path 和 adus_path 由 AI 在 bootstrap 完成后写入，无需手动填写。

**governance:audit 与 governance:generate 必须分离：**

governance:generate：
  执行 generate_adus.mjs
  功能：扫描设计文档和源代码，生成 ADUS.md 索引
  副作用：会写入 ADUS.md 文件
  用途：每次代码或文档变更后刷新索引

governance:audit：
  执行独立的审计脚本（不是 generate_adus.mjs）
  功能：只读扫描，检测治理缺口
  副作用：无（不写入任何文件）
  用途：验证规则映射、链路覆盖、格式合规性

两者不得指向同一个脚本。
将 governance:audit 指向 generate_adus.mjs 是错误的，
因为审计应该是只读的，生成器会写文件。

**步骤四：进入阶段一（文档治理）**

按第一节 1.2 的阶段一定义开始工作。
参考第二节建立项目文档体系。
参考第三节在设计文档中写入规则标注块。

阶段一完成标准：
  核心链路已在设计文档中定义（`<!-- CHAIN -->` 标注块）
  主要规则已登记，registry_status=registered
  主要执行路径无 RULE_MISSING 状态
  ADUS.md 规则注册表（自动生成区）有实质内容

**步骤五：进入阶段二（代码治理）**

按第一节 1.2 的阶段二定义开始工作。
参考第四节对代码块添加 @GOV 标注。
参考第五节处理 rules:[] 块。

阶段二完成标准见第一节 1.2。

### 0.3 最小可行接入

对于小型测试项目或验证性接入，可以使用最小化路径：

最小阶段一（1-2天）：
  定义 1-2 个核心模块编码
  定义 2-3 条链路
  定义 5-10 条核心规则
  完成一份 L1 文档和一份 L2 文档

最小阶段二（按代码量）：
  只对核心链路上的代码块标注 @GOV
  非核心链路的块可以延后处理
  每个模块完成一份 triage 报告

这个路径适合：
  验证治理流程的可行性
  团队熟悉标准的学习阶段

### 0.4 接入顺序建议

强烈建议按以下顺序接入：

1. 先用一个最小项目完整走一遍流程
   发现模板的实际问题
   修复模板
2. 再接入目标项目
   有历史代码的项目直接接入风险高
   最小项目验证过的流程更可靠

---

## 第一节：治理基础

### 1.1 核心原则（死线）

以下原则不可违反：

代码头只标注结构事实，不定性，不评价，不承诺。
boundary 字段只描述代码块实际做了什么，不评价是否正确。
规则意图由人类定义，规则ID和标注块由AI生成。
规则生成后立即可用，脚本运行后登记到注册表。
有冲突时等待人类裁决，无冲突时AI直接完成生成。
设计文档与代码冲突时，设计文档保持权威，
  不允许用 @GOV 为错误代码背书，
  不允许反向修改设计文档为错误实现开脱。
治理体系复杂度必须远低于业务复杂度。

### 1.2 两个治理阶段

**阶段一：文档治理阶段**

时机：实现开始前，设计和规划期间。
目标：定义系统应该做什么，建立规则、约束、架构决策。
信息权威：设计文档和规则注册表。
项目 APC 可以在阶段一内部进一步定义需求、设计、计划等子阶段门禁。
ADU 不规定项目必须采用何种文档子阶段。
AI 角色：生成规则标注块、执行一致性检查、自动组合映射ID。
         规则由脚本自动登记（registered）。
产出：
  模块设计文档（含 `<!-- RULE -->`、`<!-- CHAIN -->`、`<!-- CONSTRAINT -->`、
              `<!-- MODULE -->`、`<!-- DOMAIN -->` 标注块）
  ADUS.md 规则注册表条目（脚本提取后 registry_status=registered）
  ADUS.md 链路注册表条目（从 `<!-- CHAIN -->` 标注块自动提取）
  X-INDEX 约束条目
完成标准：
  核心链路已定义
  主要规则已登记（registry_status=registered）
  主要执行路径无 RULE_MISSING 状态
  ADUS.md 规则注册表（自动生成区）有实质内容

**阶段二：代码治理阶段**

时机：实现过程中及之后。
目标：验证实现是否符合设计，标注 @GOV，识别偏差。
信息权威：设计文档保持权威地位。
AI 角色：生成 @GOV 标注、执行 triage 分类、
         生成偏差报告、更新 ADUS.md。
         不独立评价实现的正确性。
产出：
  带 @GOV 标注的源文件
  ADUS.md（脚本自动生成）
  [模块]_triage.md（每模块一份）
  供人类裁决的偏差报告
完成标准：
  所有有业务规则所有权的块已声明 rules
  所有 rules:[] 块已有 triage 分类
  无未解决的 NEEDS_HUMAN_DECISION 条目
  无阻断模块治理完成的 RULE_MISSING 条目
  audit 无 CHAIN_ENTRY_UNANNOTATED error、BLOCK_UNANNOTATED error 或其他 error 级违规

### 1.3 阶段冲突解决规则

设计文档与代码实现冲突时，AI 执行以下流程：

1. AI 输出偏差报告，描述冲突点
2. 人类裁决：代码有误，或设计有误
3. 代码有误：AI 修复代码，更新 @GOV
4. 设计有误：人类更新设计文档，
             AI 更新规则注册表，
             AI 修复代码，
             AI 更新 @GOV

---

## 第二节：文档体系规范（阶段一入口）

### 2.1 文档层级

L0  治理控制文档
    ADU.md、ADP.md、CLAUDE.md（根级，项目共用）
    APC、ADUS（项目级，路径由 governance.config.json 声明）
    约束所有其他层级，不可被覆盖
    不属于项目业务文档体系

L1  项目级文档
    性质：跨模块，定义全局约束和规则
    典型内容：产品定义、总体架构、全局术语规范、项目方案
    规则可在此层定义
    每个项目至少一份

L2  模块级文档
    性质：单模块，定义模块内具体方案和规则
    典型内容：模块需求、模块技术方案、模块开发计划
    规则的主要产出层
    必须在 upstream 字段引用父级 L1 文档

L3  专项文档
    性质：对 L2 特定部分的深度拆解
    典型内容：专项设计、接口协议、数据定义、专项计划
    必须在 upstream 字段引用父级 L2 文档

L4  参考文档
    性质：只读参考，不具权威性
    文档状态只能是 R 或 X
    不能定义规则

注：各层级的具体文档名称由项目根据实际需要确定，
以上"典型内容"仅为举例，不是规定。

### 2.2 文档头标准

L0 中的 APC 和 ADUS，以及 L1/L2/L3/L4 文档，必须以以下格式的文档头开始。
ADU.md、ADP.md、CLAUDE.md 为根级治理文档，不使用文档头，引用关系在文档内容中描述。

```
---
文档编号：   [模块]-[大类]-[类型]-[序号]
文档状态：   A | R | X
负责模块：   [模块编码]
文档职责：   [一句话，不超过 20 字]
上游约束：   [约束本文的上游文档编号，无则写"无"]
直接承接：   [依赖本文的文档或实现模块]
使用边界：   [本文不覆盖的内容]
变更要求：   [修改本文时必须同步什么]
---
```

字段定义：

文档编号：
  格式：[模块]-[大类]-[类型]-[序号]
  大类：M（模块）| S（系统）| X（跨模块）| C（核心）
  类型：T（设计）| D（定义）| P（协议）| R（参考）
  序号：两位零填充
  示例：DE-M-T-01

### 2.3 文件命名规范

格式：[状态前缀]-[文档编号]_[简短名称].md

文档状态三种：
  A（Active）    约束实现，无论是否仍在活跃修改中
  R（Reference） 不约束实现，仅供参考
  X（eXpired）   已废弃，不再参考

示例：A-DE-M-T-01_差异生命周期设计.md

### 2.4 文档尾部要求

每份 A 状态文档必须以变更记录表结尾：

```markdown
## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| YYYY-MM-DD | v1.0 | 初始版本 |
```

每次修改必须新增一行。

例外：ADUS.md 自动生成区由脚本刷新，不适用本节变更记录要求。

### 2.5 文档变更传播规则

规则定义发生变化：
  AI 更新规则注册表
  AI 运行审计，查找所有引用受影响规则的 @GOV 块
  AI 输出偏差报告
  人类裁决：修复代码还是接受豁免
  AI 执行裁决结果

文档状态变为 R 或 X：
  AI 查找所有以此文档为来源的规则
  AI 输出需要审查的规则清单
  无冲突：AI 直接执行规则迁移或标记 superseded
  有冲突：AI 输出冲突报告，等待人类裁决后继续

### 2.6 APC 项目覆盖机制

ADU 定义通用治理最低结构和标注格式。
APC 定义项目级治理策略、阶段门禁、文档完整性标准和实现约束。

APC 可以收紧 ADU 标准，但不能放宽 ADU 的通用死线。

以下内容由 APC 决定：
  是否启用需求、设计、计划子阶段门禁
  是否要求每个功能提供流程图或等价流程表
  是否强制特定功能使用状态机
  是否声明项目级 SSOT
  是否要求代码规则真实性语义验证
  阶段晋级是否必须人类确认

APC 若启用上述约束，ADP 在运行时按 APC 执行阶段门禁；
ADU 只提供可复用的标注结构和静态引用标准。

---

## 第三节：规则与链路体系（阶段一核心产出）

### 3.0 模块编码规范

模块编码是规则ID、代码块ID、文档编号的基础组件。

模块编码规则：
  全大写英文字母，2-4个字符
  在项目内唯一
  反映模块的业务职责，不反映技术实现
  一旦分配不可更改（已被规则ID和代码块ID引用）

模块编码通过在设计文档中写入 <!-- MODULE --> 标注块来定义：

```
<!-- MODULE
module_code: [模块编码]
module_name: [模块名称]
description: [一句话职责描述]
-->
```

脚本运行后自动提取到模块编码表。
不得人工编辑模块编码表。

示例：
  DE    对话编辑
  WS    工作台
  AG    AI Agent
  ED    编辑器
  CORE  核心/跨模块

**MODULE 创建流程：**

1. 人类描述功能模块的业务职责
2. AI 读取业务描述，提取核心业务名词
3. AI 按以下推断规则生成 module_code：
   取核心业务名词的大写首字母缩写，2-4个字符，以字母开头，
   可包含数字（用于冲突时的序号追加，如 WS2）
   优先使用名词主体，避免泛化词（如 SVC、MGR、SYS、UTIL）
   单词较短时直接取前几个字母
   示例：Dialog Edit → DE，Workspace → WS，Agent → AG，
         Core → CORE，Editor → ED
4. AI 检查 ADUS.md 模块编码表是否已存在相同编码：
   不存在 → 直接生成 `<!-- MODULE -->` 标注块
   已存在且语义相同 → 直接复用，不新增
   已存在但语义不同 → 自动生成新编码，不打断人类：
     优先在原缩写后追加序号（如 WS → WS2）
     若追加序号仍冲突，取模块名中下一个关键字母追加
     确保新编码在模块编码表中唯一，长度不超过4个字符
5. 模块职责归属无法确定时：
   输出 MODULE_AMBIGUOUS，描述候选模块及理由，等待人类裁决
6. 标注块写入设计文档，module_code 立即可用
7. 脚本运行后自动提取，登记到 ADUS.md 模块编码表

module_code 一旦被任何规则ID或代码块ID引用，不可更改。
需要调整时新建模块编码，旧编码保留。

### 3.1 规则 ID 格式

```
BR-[模块]-[域]-[序号]    模块内业务规则
X-[类别]-[序号]          跨模块约束
```

模块编码：项目自定义
规则域：项目自定义，建议参考以下通用域：

| 域 | 含义 |
|----|------|
| STATE | 状态语义、生命周期、阶段闭合 |
| DATA | 数据结构、字段定义、schema |
| PERSIST | 持久化、恢复、重建 |
| VERIFY | 验证、审计、一致性检查 |
| GOV | 文档治理、命名、变更控制 |
| OBS | 可观测性、错误处理、恢复 |

新增规则域通过在设计文档中写入 <!-- DOMAIN --> 标注块来定义：

```
<!-- DOMAIN
domain_code: [域编码]
description: [一句话描述]
-->
```

**扩展域创建流程：**

1. 人类确认现有六个内置域无法覆盖当前规则的分类需求
2. AI 生成 `<!-- DOMAIN -->` 标注块，包含 domain_code 和 description
3. AI 检查是否与内置域或已有扩展域重复：
   语义相同：直接复用已有域，不新增
   domain_code 冲突但语义不同：在原编码后追加数字（如 DATA → DATA2）
   无冲突：写入设计文档，domain_code 立即可用
4. 脚本运行后自动提取，登记到 ADUS.md 规则域注册表

domain_code 规则：
  全大写英文字母或数字，2-6个字符，以字母开头
  一旦被规则ID引用不可更改
  废弃域保留 domain_code，不得复用

不得人工编辑规则域注册表。
序号三位零填充，从 000 开始。
规则 ID 一旦分配不可复用。
废弃规则保留 ID，registry_status 标记为 superseded。

### 3.2 设计文档中的规则标注格式

**重要：docs 格式约束**

设计文档中的规则、链路、模块、约束必须使用标注块格式。
不得使用 `## heading` 替代标注块。

原因：generate_adus.mjs 通过扫描标注块提取注册表内容。
使用 heading 格式会导致规则注册表、链路注册表、模块编码表全部为空，
generate_adus.mjs 的注册表生成功能失效。

正确格式示例（在 docs/02_RULES.md 中）：

```
<!-- RULE
rule_id: BR-TF-STATE-001
主链路: CHAIN-TASK-CREATE
域: STATE
-->
任务创建时状态必须为 draft。
```

注：registry_status 由 generate_adus.mjs 派生并写入 ADUS.md 规则注册表，
不在 `<!-- RULE -->` 标注块中手写。规则生命周期（generated / registered /
superseded）的权威值见 §3.3，由脚本管理。

错误格式示例（禁止）：

```
## BR-TF-STATE-001 任务创建规则
任务创建时状态必须为 draft。
```

规则在设计文档中使用以下结构化标注块定义：

```
<!-- RULE
rule_id: BR-[模块]-[域]-[序号]
主链路: [chain_id，多条用逗号分隔]
域: [域编码]
-->
[规则的完整描述文本]
```

标注块紧跟规则描述正文之前。
脚本扫描设计文档提取标注块，自动生成规则注册表。

链路和约束的标注块：

```
<!-- CHAIN
chain_id: [链路 ID]
主责模块: [模块编码]
status: active
risk_tier: normal
-->
```

status 合法值：active（默认）| planned（尚未实现，豁免 CHAIN_ENTRY_UNANNOTATED 检测）| superseded（废弃）。

risk_tier（可选）合法值：P0 | P1 | normal（默认）。
  P0 为高风险关键链路，P1 为次高风险链路，normal 为普通链路。
  用于 §5.5 日志覆盖分级；项目可由 APC 统一声明关键链路的 risk_tier。缺省视为 normal。

链路权威源说明：
  `<!-- CHAIN -->` 标注块是链路的唯一事实来源。
  ADUS.md 链路注册表由 generate_adus.mjs 脚本从
  设计文档的 `<!-- CHAIN -->` 标注块自动提取生成。
  不得人工编辑 ADUS.md 链路注册表。

```
<!-- CONSTRAINT
constraint_id: X-[类别]-[序号]
摘要: [一句话约束描述]
涉及模块: [模块编码，逗号分隔]
链路影响: [受影响的链路，逗号分隔]
-->
```

**CONSTRAINT 创建流程：**

1. 人类描述跨模块约束的业务场景
2. AI 生成 `<!-- CONSTRAINT -->` 标注块，
   包含 constraint_id、摘要、涉及模块、链路影响
3. AI 检查 ADUS.md 跨模块约束表是否已存在语义相同的约束
4. 无冲突：标注块写入设计文档，constraint_id 立即可用
5. 有冲突：AI 输出冲突报告，等待人类裁决后继续
6. 脚本运行后自动提取，登记到 ADUS.md 跨模块约束表

constraint_id 规则：
  格式：X-[类别]-[序号]
  推荐类别（项目可根据需要扩展）：CONST / SEQ / FORBID / SHARED / INTER
  序号三位零填充，从 001 开始
  一旦分配不可复用
  废弃约束保留原 constraint_id，在设计文档中添加注释说明废弃原因

**TERM 标注块（术语锚定）：**

TERM 用于锚定核心业务概念的唯一命名，防止跨 session 命名漂移。
只注册满足以下条件的概念：跨 session 容易漂移、代码中有实体承载、命名混乱会导致规则判断错误。
普通 UI 词、自然语言词、技术实现词不注册。

```
<!-- TERM
term_id: TERM-[链路缩写]-[三位序号]
chains: [chain_id，跨链路用逗号分隔]
zh: [正式中文名，唯一]
en: [正式英文名，同时是代码标识符基础形式，唯一]
forbidden: [禁用别名，逗号分隔，可为空]
-->
```

TERM 块必须集中放在设计文档开头的 `## 0. Terminology Registry` 区域，不得散落在正文。
只有主责模块的设计文档注册 TERM，其他文档只引用。
单份文档 active TERM 超过 12 个时 audit 给 warning（信号，非硬限制）。

**TERM 创建流程：**

1. 人类描述需要锚定的核心业务概念
2. AI 确认该概念满足注册条件：跨 session 容易漂移、代码中有实体承载、命名混乱会导致规则判断错误
   不满足条件的普通名词不注册
3. AI 生成 `<!-- TERM -->` 标注块，放入该文档的 `Terminology Registry` 区域
4. AI 检查 ADUS 术语注册表是否已存在相同 zh 或 en 且 chains 有交集的 TERM
5. 无冲突：标注块写入设计文档，term_id 立即可用
6. 有冲突：AI 输出 TERM_DUPLICATE 冲突报告，等待人类裁决后继续
7. 脚本运行后自动提取，登记到 ADUS.md 术语注册表

term_id 规则：
  格式：TERM-[链路缩写]-[三位序号]
  跨链路概念取主链路缩写，用 chains 字段声明全部关联链路
  一旦分配不可复用
  废弃 TERM 保留 term_id，在设计文档 TERM 块中添加注释说明废弃原因
  不得在 @GOV 的 term_ref 中引用已废弃的 TERM

**TERM 相关 audit 检测类型：**

| 类型 | 级别 | 触发条件 |
|------|------|---------|
| TERM_UNREGISTERED | 硬错误 | @GOV 的 term_ref 指向不存在的 TERM ID |
| TERM_DUPLICATE | 硬错误 | 两种触发：①同一 term_id 被注册多次（字面冲突）；②chains 有交集的两个 TERM，zh 或 en 相同但 term_id 不同（语义冲突） |
| TERM_ORPHAN | warning | TERM 已注册但无任何 @GOV 的 term_ref 引用（豁免：term_ref 是可选字段，历史代码和未标注块不纳入统计；仅当项目已全量完成 @GOV 标注时此 warning 有意义） |

**REQ 标注块（需求层规则）：**

REQ 用于表达需求层规则和需求来源，不是代码实现规则来源。
代码 @GOV 不得直接引用 REQ。
REQ 必须通过 REQ_MAP 或项目 APC 声明的等价映射关系连接到 RULE / CHAIN / CONSTRAINT 后，
才能进入代码治理链。

```
<!-- REQ
req_id: REQ-[MODULE]-[SEQ]
title: [需求标题]
owner_module: [MODULE]
status: active | candidate | deferred | blocked | superseded
summary: [一句话需求意图]
-->
```

REQ 必填字段不超过五个：
  req_id
  title
  owner_module
  status
  summary

REQ 可选字段：
  source_doc
  priority
  acceptance
  open_issues
  related_terms
  notes

req_id 规则：
  格式：REQ-[MODULE]-[三位序号]
  MODULE 使用已注册模块编码
  序号三位零填充，从 001 开始
  一旦分配不可复用
  废弃需求保留原 req_id，status 改为 superseded

REQ 与 ADUS 协同：
  generate_adus 可从 `<!-- REQ -->` 标注块生成需求注册表。
  需求注册表记录 req_id、owner_module、status 和 source_doc。
  REQ 注册表只提供需求追踪，不直接参与代码 @GOV rules 校验。

**REQ_MAP 标注块（需求到技术规则映射）：**

REQ_MAP 用于声明需求到技术规则的追踪关系。
技术实现规则来源仍然是 RULE / CHAIN / CONSTRAINT，不是 REQ。

```
<!-- REQ_MAP
req_id: REQ-[MODULE]-[SEQ]
target_type: RULE | CHAIN | CONSTRAINT
target_id: [BR-* | CHAIN-ID | X-*]
status: mapped | candidate | deferred | blocked | superseded
-->
```

REQ_MAP 必填字段不超过五个：
  req_id
  target_type
  target_id
  status

REQ_MAP 可选字段：
  reason
  source_doc
  notes

status 语义：
  mapped：已映射到正式技术规则
  candidate：候选映射，不得驱动运行时代码
  deferred：显式延期
  blocked：映射受阻
  superseded：已被新映射替代

REQ_MAP 与 ADUS 协同：
  generate_adus 可从 `<!-- REQ_MAP -->` 标注块生成需求映射表。
  audit 可检查 target_id 是否存在于已注册 RULE / CHAIN / CONSTRAINT。（enforcement: deferred — 当前引擎未实现该校验）
  候选映射不得被运行时代码 @GOV 引用。

**STATE_MACHINE 标注块（可选状态机索引）：**

ADU 不强制项目必须使用状态机。
当 APC、设计文档或规则声明某功能受状态机约束时，
STATE_MACHINE 标注块可作为 ADUS 状态机索引来源。
代码是否必须承接状态机，由 APC 和具体技术设计文档决定。

```
<!-- STATE_MACHINE
machine_id: SM-[MODULE]-[SEQ]
owner_module: [MODULE]
status: active | candidate | deferred | superseded
summary: [一句话说明控制范围]
source_type: design | reference | migration
-->
```

STATE_MACHINE 必填字段不超过五个：
  machine_id
  owner_module
  status
  summary
  source_type

STATE_MACHINE 可选字段：
  chain
  rules
  initial_state
  terminal_states
  context
  events
  states
  transition_table
  test_matrix
  notes

STATE_MACHINE 与 ADUS 协同：
  generate_adus 可从 `<!-- STATE_MACHINE -->` 标注块生成状态机注册表。
  @GOV 可通过可选字段引用 machine_id。
  若 APC 或技术设计文档未声明状态机为强约束，缺少 STATE_MACHINE 不应阻断审计。

### 3.2.1 链路创建流程

链路和规则可以并行定义，互相参考但不互为前提。

**链路创建流程：**
1. 人类描述业务流程意图
2. AI 生成 `<!-- CHAIN -->` 标注块，包含 chain_id、主责模块、status（默认 active）
3. AI 检查 ADUS.md 链路注册表中是否已存在相同 chain_id
4. 无冲突：标注块写入设计文档，chain_id 立即可用
5. 有冲突：AI 输出冲突报告，等待人类裁决后继续
6. 脚本运行后自动提取，登记到 ADUS.md 链路注册表

chain_id 规则：
  一旦分配不可复用
  废弃链路保留原 chain_id，在设计文档的 <!-- CHAIN --> 标注块中将 status 改为 superseded
  不得在新 @GOV 标注中引用 superseded 链路

### 3.3 规则生命周期

**generated（已生成）：**
  AI 根据人类描述的意图生成 `<!-- RULE -->` 标注块
  rule_id 已写入设计文档
  尚未被脚本提取到规则注册表

**registered（已登记）：**
  脚本扫描设计文档，提取 `<!-- RULE -->` 标注块
  rule_id 已登记到规则注册表，registry_status=registered

**superseded（已取代）：**
  规则被新规则取代
  旧 rule_id 保留，registry_status=superseded
  引用此 rule_id 的 @GOV 块由 AI 自动更新
  不得在新 @GOV 标注中引用

**规则创建流程：**
1. 人类在设计文档中描述规则意图
2. AI 生成 `<!-- RULE -->` 标注块，包含 rule_id、主链路、域
3. AI 执行一致性检查（见 3.4 节）
4. 无冲突：标注块写入设计文档，状态为 generated，rule_id 立即可用
5. 有冲突：AI 输出冲突报告，等待人类裁决后继续
6. 脚本运行后自动提取，登记到规则注册表，状态变为 registered

**规则变更流程：**
1. 人类描述变更意图
2. AI 更新设计文档中的 `<!-- RULE -->` 标注块
3. AI 执行一致性检查
4. 无冲突：AI 审计所有引用此 rule_id 的 @GOV 块，
           输出需要更新的块清单，直接更新受影响的 @GOV 块
5. 有冲突：AI 输出冲突报告，等待人类裁决
           人类裁决后，AI 按裁决结果执行第4步的审计和更新动作

### 3.4 规则生成协议

AI 根据人类描述的设计意图，自动生成规则标注块，输出以下格式：

```
【规则生成报告】
生成 ID：     BR-[模块]-[域]-[序号]
主链路：      [chain_id]
域：          [域编码]
一致性检查：  通过 / 冲突（见冲突详情）
冲突详情：    [若有冲突，描述冲突内容]
```

无冲突时，直接写入设计文档，规则状态为 generated，rule_id 立即可用。
脚本运行后自动提取并登记到规则注册表，状态变为 registered。
有冲突时，暂停并等待人类裁决。

一致性检查内容：
1. 无语义相同或重叠的已有规则
   （ADUS.md 规则注册表为空时，此项自动通过）
2. 规则域与模块责任匹配
3. 规则能映射到至少一条链路
   （链路注册表为空时，此项允许标注为"待链路定义后验证"，
   不阻断规则生成）
4. 规则不与任何跨模块约束矛盾
   （跨模块约束表为空时，此项自动通过）

注：原第4条"规则执行在代码中可观察"属于阶段二验证项，
不在阶段一规则生成时执行。

任何检查失败输出冲突详情，等待人类解决后继续。

---

## 第四节：代码头规范（阶段二入口）

> ADP 引用点：本节是 ADP GOVERNANCE_UPDATE_GATE 的格式依据。

### 4.1 目的与定位

**@GOV 语法规范（强制）：**

@GOV 标注必须使用块注释格式，不得使用行注释格式。

正确格式：
```
/**
 * @GOV
 * codes:    [复合映射ID]
 * type:     [类型]
 * chain:    [链路ID]
 * rules:    [规则ID列表]
 * boundary: in=[输入] | out=[输出]
 */
```

禁止格式：
```
// @GOV
// codes: ...
```

原因：generate_adus.mjs 和 governance:audit 脚本统一识别块注释格式。
使用行注释会导致脚本无法解析，@GOV 标注失效。

@GOV 是代码块的结构坐标系统。它回答：

```
这个代码块是什么？
它属于哪条链路？
它受哪些规则约束？
它实际做了什么？
```

@GOV 不是质量评价。
@GOV 不是合规声明。
@GOV 不是动态状态记录。

### 4.2 五个必填字段

字段必须按以下顺序出现，不能增减：

```
codes
type
chain
rules
boundary
```

**codes 字段**

定义：复合映射ID，表达规则到代码块的有向连接关系。

格式：[规则ID]-[代码块ID]
代码块ID格式：[TYPE]-[MODULE]-[CHAIN]-[SEQ]

一个代码块可以有多个映射ID，逗号分隔。
每个映射ID对应一条规则与此代码块的连接关系。
每个映射ID是自描述的，无需查阅其他文档即可理解连接关系。

示例：
```
codes: BR-DE-DIFF-002-EFFECT-DE-DIFF-ACCEPT-001,
       BR-DE-DIFF-008-EFFECT-DE-DIFF-ACCEPT-001
```

UTIL块例外：codes只有代码块ID，无规则ID前缀。
示例：codes: UTIL-DE-TEXT-NORMALIZE-001

规则：
  代码块ID部分在整个代码库内唯一
  一旦分配不可复用
  废弃块保留原codes，在triage报告中标记
  代码块ID是稳定坐标，不因规则变化而重命名
  规则ID部分必须处于 generated 或 registered 状态
  注：codes 和 rules 字段均允许引用 generated 或 registered 状态的规则。
      registry_status=superseded 的规则不得出现在 codes 或 rules 字段中。
      若引用的规则仍为 generated 状态，应在标注完成后尽快运行
      `npm run governance:generate` 将其推进到 registered。

**type 字段**

必须是以下八种之一：
`DATA` `GUARD` `EFFECT` `RB` `API` `UTIL` `IO` `QUERY`
详见第 4.3 节。

**chain 字段**

一个或多个链路 ID，逗号分隔。
链路 ID 必须存在于 ADUS.md 链路注册表中。
若无合适链路，先在设计文档中写入 `<!-- CHAIN -->` 标注块，
脚本提取后再写入代码头。

**rules 字段**

辅助字段，列出此块直接相关的规则 ID。
供 AI 在修改时快速感知共用规则，防止专项修复破坏其他消费链。
格式：`BR-[模块]-[域]-[序号]` 或 `X-[类别]-[序号]`
rules 字段可引用 registry_status 为 generated 或 registered 的规则 ID。
registry_status=superseded 的规则不得出现在 rules 字段或 codes 字段中。
可为空：`rules: []`
空规则处理见第五节。

**boundary 字段**

结构化事实声明，详见第 4.6 节。

**term_ref 字段（可选）**

引用已注册的 TERM ID，声明此代码块的核心业务概念归属。

```
term_ref: TERM-[链路缩写]-[三位序号]
term_ref: TERM-[链路缩写]-[三位序号], TERM-[链路缩写]-[三位序号]
```

规则：
  term_ref 指向的 TERM ID 必须已在 ADUS.md 术语注册表中登记（TERM_UNREGISTERED 为硬错误）
  一个代码块涉及多个核心概念时，可用逗号分隔列出多个 TERM ID，audit 逐个校验
  新代码涉及已注册 TERM 的核心概念时建议填写
  不强制回填历史代码
  可为空，不填写不报错，但 TERM_ORPHAN 的检测覆盖率会降低

### 4.3 八种代码块类型

**DATA**
  定义：类型、接口、联合类型、schema、枚举。纯结构，不执行逻辑。
  rules 分配：只挂字段存在性规则和字段语义约束规则。
  boundary 要求：`in=` 描述生产者，`out=` 描述类型结构概要。

**GUARD**
  定义：纯判断函数。返回 boolean，无副作用。
  rules 分配：只挂直接定义判断条件的规则，最多 2 条。
  boundary 要求：`out=` 必须写成 `boolean（true 当 [条件]）`

**EFFECT**
  定义：产生可观察副作用的代码块。
  四种子类型通过 boundary 的 `effect=` 声明：
    `memory`  store 状态变更
    `DB`      数据库读写
    `IPC`     跨进程调用
    `WS`      响应外部事件
  注意：type 字段只写 `EFFECT`，不写 `EFFECT[memory]`。
  rules 分配：只挂与副作用职责直接匹配的规则。
  boundary 要求：`does_not=` 至少一条（必填）

**RB**
  定义：业务流程协调者。委托给其他类型执行。
  rules 分配：只挂该块直接拥有的业务规则。
              被调用的 GUARD 或 EFFECT 已声明的规则不得重复。
  boundary 要求：标准 `in=`/`out=`，委托时使用 `delegate=`。

**API**
  定义：协议边界，跨系统接口定义。
  rules 分配：只挂协议边界规则。
  boundary 要求：必须声明 `effect=IPC-[命令名]`。

**UTIL**
  定义：纯工具函数。必须同时满足：
    1. 无副作用
    2. 无可变状态
    3. 无业务语义
    4. 不依赖 mock 可独立测试
  rules 分配：`rules: []` 永远为空。
  boundary 要求：`in=`、`out=`、`does_not=own_decision`（必填）

**IO**
  定义：持久化层文件读写操作，或适配层命令分发，无业务判断，不拥有规则。
  适用场景：存储层 save/write 操作、CLI 命令分发路由。
  rules 分配：`rules: []`（规则由调用方持有）。
  boundary 要求：`in=`/`out=` 描述操作目标（如 `out=tasks.json write`）。

**QUERY**
  定义：只读数据访问。返回数据集合，无副作用，不修改任何状态。
  适用场景：列表查询、按 ID 查找、过滤查询。
  rules 分配：只挂约束查询行为的规则（如只读约束、过滤规则）。
  boundary 要求：标准 `in=`/`out=`，`out=` 描述返回集合结构。

**类型判定决策树**（按顺序，首个匹配即停止）：

```
1. 是 interface/type/union/enum/schema 定义？→ DATA
2. 返回 boolean 且无副作用？→ GUARD
3. 是跨系统协议边界定义？→ API
4. 主要作用是跨进程调用？→ EFFECT（子类型 IPC）
5. 包含 store 状态变更？→ EFFECT（子类型 memory）
6. 响应外部系统事件？→ EFFECT（子类型 WS）
7. 有副作用的文件读写或适配层分发，无业务判断？→ IO
8. 只读数据访问，无副作用？→ QUERY
9. 满足 UTIL 全部四个条件？→ UTIL
10. 其他 → RB
```

**@GOV 强制标注范围（mandatory scope）：**

并非所有导出符号都必须有 @GOV。强制层级如下：

强制（缺失即 error）：
  - 任一已实现 CHAIN 的入口块缺少 `@GOV chain=` 引用 → `CHAIN_ENTRY_UNANNOTATED`（见 §4.5）。

应当标注（缺失为 warning，由 triage 裁决，不阻断交付）：
  - 承载业务规则或副作用的导出可执行块（GUARD / EFFECT / RB / API / IO / QUERY）。
  - 缺失时 audit 报 `BLOCK_UNANNOTATED` warning（见 §5.1），在 triage 中分类处理。

不要求 @GOV：
  - 纯展示型 UI 组件：无业务规则、无副作用、仅依据 props 渲染。
  - 情况二薄包装函数：由被委托块的 @GOV 覆盖（见 §4.5）。
  - 纯类型定义（DATA）可按需标注，不强制。

人类可对特定改动明确豁免 @GOV 标注或轻量化治理流程；
豁免须记入 triage 报告或 Task Runtime Report 的 `unresolved_items`，不得隐式省略。

### 4.4 codes 字段：复合映射 ID 规范

复合映射ID的含义：
  [规则ID]-[代码块ID] 表示"规则约束了此代码块"这条有向连接。
  一个代码块受几条规则约束，就有几个映射ID。

代码块ID生成规则：
  TYPE：与 type 字段保持一致
  MODULE：所属模块编码
  CHAIN：主链路缩写
  SEQ：三位零填充序号，从 001 开始，
       同一模块内同一 TYPE 的 SEQ 不重复。
       即 EFFECT 类型在同一模块内从 001 顺序编号，
       GUARD 类型单独从 001 顺序编号，两者互不影响。

映射ID生成时机：
  规则ID来自 generated 或 registered 状态的规则
  代码块ID在阶段二代码实现时生成
  两者由AI自动组合形成映射ID，写入 codes 字段

### 4.5 rules 字段分配原则

**核心原则：**
  每条规则有且只有一个主责块（primary owner）。
  主责块是直接实现或强制执行该规则核心约束的块。

**主责块判定标准（多个块都涉及同一规则时）：**
  取调用链中最内层、最直接执行约束的块，而非上层协调者。
  判断顺序：
    1. GUARD 块直接定义判断条件 → 该 GUARD 是主责块
    2. EFFECT 块直接执行副作用约束 → 该 EFFECT 是主责块
    3. DATA 块直接定义字段存在性或语义 → 该 DATA 是主责块
    4. 若以上均无，RB 块作为最近协调者 → 该 RB 是主责块
  上层 RB 调用已有主责块时，不得再次声明自己是同一规则的主责块。
  无法唯一确定时，输出 MODULE_AMBIGUOUS 等待人类裁决。

同一条规则可以出现在非主责块中，当该块是：
  规则输出的消费者（consumer）
  规则条件的检查者（enforcer）
  规则执行的触发者（trigger）

重复出现时必须在 boundary 中通过以下 key 说明：
  `consumer=`、`delegate=` 或 `caller_context=`

无法说明的重复是映射错误，AI 在 triage 中修正。

**责任层级：**
```
DATA 层：  字段存在性规则、字段语义约束规则
GUARD 层： 判断条件规则
EFFECT 层：副作用规则、持久化规则
RB 层：    子块未覆盖的业务协调规则
API 层：   协议边界规则
UTIL 层：  永远不拥有规则
```

**委托规则：**
  RB 调用已有 GUARD 块时，该 GUARD 的规则不得出现在 RB.rules 中。
  RB 调用已有 EFFECT 块时，该 EFFECT 的规则不得出现在 RB.rules 中。
  委托关系通过 boundary 的 `guard=` 或 `delegate=` 表达。

**规则数量参考**（信号，非硬限制）：
```
DATA：   1-3 条
GUARD：  1-2 条
EFFECT： 1-3 条
RB：     1-4 条（排除委托后）
API：    1-2 条
UTIL：   0 条（永远）
```
超过参考范围时，检查是否存在层级错误或职责过重。

**薄包装函数的 @GOV 处理规则：**

当函数 A 通过委托调用函数 B 实现时，
按以下标准判断是否需要独立 @GOV：

情况一：函数 A 代表独立的业务链路入口
  需要独立 @GOV。
  chain 字段写函数 A 所属的链路（不是函数 B 的链路）。
  rules 只挂函数 A 作为该链路入口时直接拥有的规则。
  不重复函数 B 的 @GOV 中已声明的规则。
  在 boundary 中用 delegate= 说明委托关系。

  判断标准：
    函数 A 是某条 CHAIN 的第一个入口块
    或函数 A 在链路视图中代表一条独立链路的起点

情况二：函数 A 只是函数 B 的参数变体或语法糖
  不需要独立 @GOV。
  由函数 B 的 @GOV 覆盖。
  在函数 B 的 boundary 中用 delegate= 说明。

  判断标准：
    函数 A 不代表任何独立的 CHAIN 入口
    函数 A 的存在只是为了简化调用方的参数传递

判断模糊时：
  检查链路注册表：若存在对应的 CHAIN，按情况一处理。
  若无对应 CHAIN，按情况二处理。

**链路入口强制标注规则（CHAIN_ENTRY_UNANNOTATED）：**

每条已实现的 CHAIN 必须至少有一个代码 `@GOV` 块以 `chain=` 字段引用该链路 ID，
否则视为链路入口未标注。

audit 脚本交叉比对 CHAIN 注册条目与代码 @GOV 标注：
  当前引擎实现「覆盖式」检测：某条 CHAIN 无任何 `@GOV` 块以 `chain=` 引用时，
  报 `CHAIN_ENTRY_UNANNOTATED` error。
  入口函数粒度的区分（入口函数存在但无 @GOV / 存在但无 chain= 引用 → warning）
  尚未在引擎实现，属 deferred 检测项。

豁免：链路注册表中标注为 `status: planned`（尚未实现）或 `status: superseded`（已废弃）
的链路不纳入检测。

### 4.6 boundary 字段规范

**目的：**
  声明代码块真实行为的结构事实。
  不评价行为是否正确。

**语法：**
```
boundary: key=value | key=value | key=value
```
长行可换行，续行缩进 10 个空格。

**各类型必填 key：**

| 类型 | 必填 key |
|------|---------|
| 所有类型 | `in=`（输入描述）、`out=`（输出描述） |
| EFFECT | `does_not=`（至少一条明确排除项） |
| GUARD | `out=boolean（true 当 [精确条件]）` |
| EFFECT/IPC | `effect=IPC-[命令名]` |
| EFFECT/DB | `effect=DB-[操作名]` |

**条件必填 key：**
```
在 store subscribe() 回调内执行：
  caller_context=store_subscriber

在 subscriber 内调用 store.set()：
  runtime_guard=no_reentrant_set

调用跨进程 API：
  caller_context=platform_runtime

catch 块：
  error_flow=（必须描述捕获范围）
```

**可选 key：**
```
caller_context=
  可选值：store_subscriber | react_effect | async_chain
          | platform_runtime | sync_call

runtime_guard=
  描述运行时必须满足的条件（事实，不是承诺）

coordinate_space=
  可选值：ProseMirror | markdown_offset | char_offset

document_snapshot=
  可选值：pre-update | post-update | stable

consumer=

error_flow=
  可选值：propagates | caught_by=[位置] | silent_[原因]

lifecycle_assumption=

scope=
  可选值：config_only | test_only | generated | platform_only

delegate=

effect=
  可选值：IPC-[命令] | DB-[操作] | memory | WS-[事件]
```

**死线 — boundary 绝对禁止：**
```
compliant、valid/invalid（独立词）、ensures、guarantees
should、must fix、already handled、implementation gap
correct/incorrect、properly、safely
任何动态运行状态的当前值描述
任何对行为正确性的评价
in= 或 out= 使用纯抽象占位词（data / input / output / result / state / value / object / item / payload / request / response / params / param / info）
```

BOUNDARY_ABSTRACT 说明：
  `in=` 和 `out=` 必须包含具体业务实体名或类型名，不得使用无业务语义的占位词。
  audit 脚本检测 boundary 中 `in=` 或 `out=` 后紧跟纯抽象词，发现则报 BOUNDARY_ABSTRACT。
  正确示例：`in=ValidatedPatchRange | out=DiffCardState`
  错误示例：`in=data | out=result`（触发 BOUNDARY_ABSTRACT）

DELEGATE_ABSTRACT 说明：
  `delegate=` 必须是当前块实际调用的直接下游标识符（函数名、服务名、状态机实例名）。
  不得填写架构层描述词（如状态机类型名、模块名、概念名）。
  audit 脚本检测 delegate= 值是否为已知业务标识符，无法验证时报 DELEGATE_ABSTRACT warning。（enforcement: deferred — 当前引擎未实现该检测）
  正确示例：`delegate=agentService.sendMessage | delegate=editorMachine`（实例引用）
  错误示例：`delegate=四个状态机`、`delegate=state machines`（触发 DELEGATE_ABSTRACT）

禁词匹配规则：按完整独立词匹配，不匹配子串。
`ValidatedRange`、`invalidated` 是技术标识符，不受限制。

### 4.7 完整示例

**示例一：GUARD 块**

```typescript
/**
 * @GOV
 * codes:    BR-[MODULE]-[DOMAIN]-008-GUARD-[MODULE]-[CHAIN]-001
 * type:     GUARD
 * chain:    [CHAIN-A], [CHAIN-B]
 * rules:    BR-[MODULE]-[DOMAIN]-008
 * boundary: in=[状态枚举类型]
 *           | out=boolean（true 当 [状态A] 或 [状态B]）
 */
```

**示例二：EFFECT/memory 块**

```typescript
/**
 * @GOV
 * codes:    BR-[MODULE]-[DOMAIN]-002-EFFECT-[MODULE]-[CHAIN]-001,
 *           BR-[MODULE]-[DOMAIN]-008-EFFECT-[MODULE]-[CHAIN]-001
 * type:     EFFECT
 * chain:    [CHAIN-A]
 * rules:    BR-[MODULE]-[DOMAIN]-002, BR-[MODULE]-[DOMAIN]-008
 * boundary: in=[输入参数描述]
 *           | out=[状态字段]→[新值]
 *           | effect=memory
 *           | guard=[调用的判断函数名]
 *           | does_not=persist
 *           | does_not=write_disk
 *           | error_flow=propagates
 */
```

**示例三：EFFECT/IPC 块**

```typescript
/**
 * @GOV
 * codes:    BR-[MODULE]-[DOMAIN]-001-EFFECT-[MODULE]-[CHAIN]-001
 * type:     EFFECT
 * chain:    [CHAIN-A]
 * rules:    BR-[MODULE]-[DOMAIN]-001
 * boundary: in=[输入类型], [路径参数]
 *           | out=invoke("[命令名]")
 *           | effect=IPC-[命令名]
 *           | caller_context=platform_runtime
 *           | does_not=validate_input
 *           | does_not=modify_store_state
 *           | error_flow=propagates
 */
```

**示例四：RB 块（含委托）**

```typescript
/**
 * @GOV
 * codes:    BR-[MODULE]-[DOMAIN]-005-RB-[MODULE]-[CHAIN]-001
 * type:     RB
 * chain:    [CHAIN-A]
 * rules:    BR-[MODULE]-[DOMAIN]-005
 * boundary: in=[输入参数]
 *           | out=[输出描述]
 *           | guard=[被调用的 GUARD 函数名]
 *           | delegate=[被委托的 EFFECT 函数名]
 *           | does_not=modify_[某类内容]
 */
```

**示例五：DATA 块（含坐标语义）**

```typescript
/**
 * @GOV
 * codes:    BR-[MODULE]-[DOMAIN]-017-DATA-[MODULE]-[CHAIN]-001
 * type:     DATA
 * chain:    [CHAIN-A], [CHAIN-B]
 * rules:    BR-[MODULE]-[DOMAIN]-017
 * boundary: in=[生产者描述]
 *           | out=[双坐标结构描述]
 *           | coordinate_space=[坐标空间]
 *           | document_snapshot=pre-update
 *           | lifecycle_assumption=[隐性假设描述]
 *           | consumer=[消费方描述]
 */
```

**示例六：UTIL 块**

```typescript
/**
 * @GOV
 * codes:    UTIL-[MODULE]-[CHAIN]-001
 * type:     UTIL
 * chain:    [CHAIN-A]
 * rules:    []
 * boundary: in=[输入描述]
 *           | out=[输出描述]
 *           | does_not=own_decision
 */
```

注：UTIL 块无规则约束，codes 只有代码块 ID，无规则 ID 前缀。

---

## 第五节：无规则块处理（Triage）

> ADP 引用点：本节分类规则是 ADP VALIDATION_GATE 和 Stop Conditions SC-03/SC-04 的判断依据。

### 5.1 核心原则

`rules: []` 允许存在。
无法解释的 `rules: []` 不允许存在。

**audit 两种必须执行的扫描模式：**

正空间扫描（declaration-driven）：
  对已有 @GOV 块逐一验证合规性（字段格式、boundary 规范、rules 引用有效性等）。

负空间扫描（coverage-driven）：
  从代码出发，扫描导出符号（exported function / exported component）和 ADUS 链路注册表中登记的入口函数。
  对在扫描范围内但既无 @GOV 又无 triage 记录的代码块，报 `BLOCK_UNANNOTATED` warning。
  对 ADUS 链路入口函数缺失 @GOV 或 @GOV 中无 `chain=` 引用的，报 `CHAIN_ENTRY_UNANNOTATED` error/warning（规则见 §4.5）。

两种模式都是 `governance:audit` 的必须执行项，不是可选项。
仅执行正空间扫描的 audit 不满足本规范。

每个 `rules: []` 块必须在 triage 报告中有分类记录。
分类信息不写入 @GOV 代码头。

### 5.2 七种分类及处理方式

**VALID_EMPTY（有效空规则）**
  适用：纯工具函数，无业务规则所有权，
        满足 UTIL 全部四条。
  AI 自主处理：
    保留 `rules: []`
    boundary 加入 `does_not=own_decision`
    在 triage 报告中记录原因

**CALLER_OWNS_RULE（调用方持有规则）**
  适用：当前块只执行基础操作，
        业务规则所有权完全属于调用方，内部无策略判断。
  AI 自主处理：
    保留 `rules: []`
    boundary 加入 `consumer=` 或 `caller_context=`
    在 triage 报告中记录

**ADAPTER_ONLY（仅适配层）**
  适用：IPC 包装或平台桥接，只包含跨进程调用，无业务条件判断。
  AI 自主处理：
    保留 `rules: []`
    boundary 加入 `effect=IPC-[命令]` 或 `caller_context=platform_runtime`
    在 triage 报告中记录

**RULE_MISSING（规则缺失）**
  适用：块有业务决策权，但规则文档中无对应规则。
  AI 处理：
    不添加规则 ID
    在 triage 报告中标记为 rule_gap
    暂停此块的治理
    输出提示：需要在设计文档中补充规则定义
  此分类阻断模块治理完成，需人类介入定义规则后继续。

**RULE_UNMAPPED（规则未挂载）**
  适用：规则已存在，但该块的 rules 字段遗漏了。
  AI 自主处理：
    立即在 rules 字段补充规则 ID
    更新 codes 字段补充对应映射 ID
    更新 triage 报告

**LEGACY_COMPAT（历史兼容层）**
  适用：历史兼容 shim，不在主要执行路径上。
  AI 处理：
    生成退出策略草案，包含：
      `activation_condition`：何时仍需要此 shim
      `exit_condition`：满足什么条件可以删除
      `exit_validation`：用什么测试证明可以安全删除
    将草案写入 triage 报告
    同时输出提示：LEGACY_COMPAT 退出策略已生成，请人类审阅确认
    无法生成退出策略时，升级为 NEEDS_HUMAN_DECISION

**NEEDS_HUMAN_DECISION（需要人工裁决）**
  适用：无法自动分类，需要产品或架构判断。
  AI 处理：
    冻结该块的治理
    在 triage 报告中记录具体问题描述
    输出提示：等待人类裁决
  不得跨两个治理周期持续存在。

**BLOCK_UNANNOTATED（未标注代码块）**
  适用：函数或组件在以下任一检测范围内，但既无 @GOV 又无 triage 记录：
    - 导出函数（export function / export const）
    - 导出组件（export default / export const [大写开头]）
    - ADUS 链路注册表中出现的函数名

  豁免（以下情况不纳入检测）：
    - 纯类型声明（interface / type / enum）
    - 测试文件内函数
    - 只做参数透传的工厂函数（判断标准：函数体只有一条 return，且无条件分支）

  AI 处理：
    对检测范围内的未标注块，在 triage 报告中逐一记录
    同时判断其应归属的分类（VALID_EMPTY / CALLER_OWNS_RULE / RULE_MISSING 等）
    不得将「暂时找不到对应规则」直接记录为 BLOCK_UNANNOTATED，须进一步分类
  audit 脚本对在范围内但既无 @GOV 又无 triage 记录的代码块，报 BLOCK_UNANNOTATED warning。

### 5.3 triage 报告格式

triage 报告是独立文件：`[模块]_triage.md`
不是 ADU.md 或 ADUS.md 的一部分。

每条记录格式：
```
codes:          [代码块 codes 值]
文件:           [相对文件路径]
分类:           [七种分类之一]
原因:           [一句话说明]
已采取动作:     [AI 做了什么]
需要人类介入:   是 / 否
已解决:         是 / 否
```

### 5.4 豁免 @GOV 的文件类型

以下文件完全跳过 @GOV 标注和 triage：

```
*.test.ts / *.spec.ts / *.test.tsx / *.spec.tsx
*.d.ts
*.config.ts / *.config.js
**/generated/**
**/__mocks__/**
**/build/** / **/dist/**
**/node_modules/**
```


### 5.5 日志代码处理

日志不标注 @GOV，通过链路覆盖率要求管理。

**risk_tier=P0 / P1 链路**（高风险，低频；risk_tier 在 CHAIN 标注块声明，见 §3.2）：
  入口点必须有一条 debug 日志
  出口点必须有一条 debug 日志
  每条错误流路径必须有一条 debug 日志

**高频路径**（渲染/subscriber/onUpdate）：
  只要求错误流路径有日志

**所有 catch 块：**
  必须有一条日志语句
  静默 catch 是治理违规

日志格式：`[模块-链路] 动作: 描述`

### 5.6 测试文件处理

测试文件不标注 @GOV；测试治理通过“稳定策略 + 动态资产”接入治理系统：

- ADU 定义跨项目不变的映射、发现、覆盖和失败关闭规则。
- APC 定义项目风险、阶段深度、硬门和例外边界。
- `governance.config.json.test_governance_path` 指向项目测试治理注册表；注册表保存模块源码路径、
  测试路径、风险等级、直接消费者、测试文件基线与整合目标。
- ADP 在每次任务的 VALIDATING 阶段消费变更影响计划，并在 Runtime Report 留存实际选择和结果。
- ADUS 的“测试能力索引”由当前文件系统动态生成，不人工维护测试文件名单。

测试来源不是测试文件本身，也不是仅凭需求标题推断。候选测试必须由以下至少一个来源产生：

1. 已注册 RULE 的可观察行为、边界、异常和不变量。
2. CHAIN / CONSTRAINT / STATE_MACHINE 的步骤、转移、跨模块契约与恢复路径。
3. 模块公开功能、组件交互状态、服务副作用和历史缺陷回归。
4. 本次变更文件及其所属模块、直接消费者和风险等级。

需求及验收标准用于验证“产品是否满足意图”；上列技术事实用于测试“实现是否保持可执行约束”。
二者可在同一开发循环连续执行，但证据必须分别记录。能稳定自动化的细节（包括按钮单次交互后的
前端状态变化）应下沉到最小可靠层；只有依赖主观观感、真实外设、签名/发布环境等内容才进入人工验证。

测试层级与默认映射目标：

- `unit`：纯逻辑、数据转换、小粒度交互决策。
- `state`：状态机转移、store 状态与不变量。
- `service`：I/O 边界、命令编排、错误恢复和副作用。
- `component`：用户事件到可观察 UI 状态，不以像素级实现细节为默认断言。
- `integration`：跨模块 CHAIN / CONSTRAINT 与前后端契约。
- `system`：关键需求路径、发布环境与完整产品结果；可自动部分优先自动化。

测试文件内允许轻量注释，供脚本提取规则覆盖信息：
```typescript
// covers: BR-[模块]-[域]-[序号]
```

格式规则：
  注释必须独占一行
  rule_id 必须与规则注册表中的格式完全一致
  一行只写一个 rule_id
  多条规则分多行写

动态发现与归属规则：

- 前端发现 `*.test.*` / `*.spec.*`；Rust 发现 `#[cfg(test)]` 内联测试与 `src-tauri/tests/*.rs`。
- 每个测试资产必须有模块 owner 和 layer；`covers` 可建立附加规则映射。
- 每个可执行源文件必须能映射到模块。无法映射的新文件按失败关闭处理，不得静默跳过。
- 直接修改测试时只运行该测试；修改业务源时选择所属模块，并对 critical/high 风险模块扩展一跳直接消费者。
- 共享配置、Cargo/Rust、高风险跨层或影响范围过大时自动升级为全量回归。
- `only` / `skip` / Rust `#[ignore]` 默认是治理失败，不得作为临时通过手段。
- 文件数量以注册表 baseline 作为“禁止继续增长”的硬线，以 target 作为整合债务；目标不是机械地每模块
  只留一个大文件，而是在可诊断性和加载成本允许时收敛到一个或少数稳定套件。

测试辅助代码无需逐条规则映射，但仍必须由某个已归属测试资产使用。新增或删除功能时，应同步更新
技术设计/模块路径与测试；动态清单会自动增删测试资产，`governance:audit` 负责发现无 owner、无 layer、
未知 `covers`、文件基线增长或可执行源未映射等漂移。

**链路集成测试声明规范：**

ADUS 链路注册表中每条链路必须有 `integration_test` 字段，不允许空值静默通过 audit。

合法值：
  `[测试文件路径]::[测试名称]`   — 已有集成测试，路径可验证
  `deferred: phase-N`             — 当前阶段有意推迟，N 为预计阶段编号

audit 检测规则（enforcement: deferred — 当前引擎未实现 CHAIN_INTEGRATION_TEST_MISSING 检测）：
  `integration_test` 字段缺失或为空 → 报 `CHAIN_INTEGRATION_TEST_MISSING` warning
  `deferred` 值存在 → 通过 audit，但在 audit 报告中列出所有 deferred 条目供人类确认优先级

`deferred` 不是永久豁免，只是显式推迟声明；进入对应阶段时必须补充实际测试并移除 `deferred` 标记。

### 5.7 AI 语义审计

`governance:audit` 脚本输出 `semantic_review_candidates` 列表，包含所有有规则引用且 boundary 非空的 @GOV 块。
这是需要 AI 介入做语义比对的候选集，脚本本身不做判断。

**AI 语义审计流程：**

1. 读取 `semantic_review_candidates` 列表
2. 对每个候选块：读取其引用规则的文档原文，对比 boundary 描述中的核心实体名称
3. 发现名词不一致 → 输出结构化偏差报告（格式见下）
4. 无法判断 → 标记 `confidence: low`，输出 `recommended_action: human_review`

**AI 语义审计输出格式（每个发现一条）：**

```json
{
  "finding": "possible_term_drift",
  "rule_id": "BR-DE-DIFF-018",
  "expected_terms": ["DiffCard", "GreenReview"],
  "observed_terms": ["ReviewItem", "PatchCard"],
  "evidence": [
    "boundary uses ReviewItem",
    "rule text uses DiffCard",
    "implementation does not mention GreenReview"
  ],
  "confidence": "high | medium | low",
  "recommended_action": "fix_naming | human_review"
}
```

`confidence: low` 时只输出报告，不输出判断结论，直接标记 `recommended_action: human_review`。

**语义审计触发点：**

Issue Trace 阶段：输出 `semantic_review_candidates` 候选列表，供后续消费。
VALIDATING 阶段（ADP）：若本次 CODE_CHANGE 涉及有 `semantic_review_candidates` 标记的规则，必须执行 AI 语义比对，将发现写入 Task Runtime Report 的 `unresolved_items` 或 `stop_conditions`。
  候选列表不得在 VALIDATING 阶段静默跳过。
  无候选条目时可跳过此步骤。

### 5.8 新概念候选暴露（new_domain_concept_candidate）

`governance:audit` 扫描源代码中新增的 exported 实体（type / interface / class / enum），
检查名称是否出现在 ADUS 术语注册表的 en 字段中。

未出现者输出到 `new_domain_concept_candidates` 列表，供 AI 和人类判断：
- 合理新概念 → 在设计文档补充规则定义和 TERM 注册
- 命名漂移 → 修正名称

此检测为 informational，不阻断 audit 通过。

---

## 第六节：修改前溯源协议（Issue Trace）

> ADP 引用点：本节内容（触发条件与十一项）是 ADP TRACE_GATE 的执行依据。
> ADP 定义执行时序，ADU 定义执行内容，两者不重写对方。

### 6.0 Issue Trace 前置条件

执行 Issue Trace 前，必须满足以下条件：

**必须已完成：**
  ADUS.md 模块编码表有实质内容
  （至少一个 `<!-- MODULE -->` 标注块已被脚本提取）
  ADUS.md 规则域注册表有实质内容
  至少一条链路已在设计文档中定义并被脚本提取
  至少一条规则已达到 registered 状态

**空库降级路径：**
  若 ADUS.md 自动生成区为空（阶段一尚未完成），
  Issue Trace 的以下项目允许填写"暂无"：
    第3项（相关规则）：填写"规则注册表为空，阶段一未完成"
    第5项（生产者块）：填写"@GOV 标注尚未建立"
    第6项（消费者块）：填写"@GOV 标注尚未建立"
    第7项（空规则块）：填写"@GOV 标注尚未建立"

  空库降级路径仅适用于对已有遗留代码库的首次治理接入。
  新项目必须完成阶段一后再进入阶段二，不得使用降级路径。

### 6.1 触发条件

**必须执行 Issue Trace 的情况：**
- 任何 bug 修复
- 任何涉及 @GOV 块的重构
- 任何向已有链路添加新块的功能开发
- 任何影响已有 @GOV 声明的规则更新
- 任何模块治理审计

**不需要 Issue Trace 的情况：**
- 只添加新的 UTIL 块（rules: []）
- 只修改测试文件
- 只修改文档
- 只修改日志语句

### 6.2 十一项格式

AI 必须按顺序输出全部十一项，不得跳过。

**1. 用户可见症状**
   用户观察到什么现象？1-3 句话。
   若无用户可见症状（重构/审计），明确说明。

**2. 入口点**
   哪个函数、事件或用户动作启动了此流程？
   需要文件路径和函数名。

**3. 相关规则**
   列出约束此流程的所有 BR-* 和 X-* 规则 ID。
   来源：在规则注册表中检索相关链路 ID。
   若找不到规则：明确标注 RULE_MISSING。

**4. 相关链路**
   列出此次修改涉及的所有链路 ID。
   若有顺序关系，用箭头表示：CHAIN-A → CHAIN-B。

**5. 生产者块**
   列出为此流程生产数据的所有 @GOV 块。
   格式：`[codes] 位于 [文件路径]`

**6. 消费者块**
   列出消费此流程输出的所有 @GOV 块。
   格式：`[codes] 位于 [文件路径]`

**7. 路径上的空规则块**
   列出执行路径上所有 `rules: []` 的块及其 triage 分类。
   未分类的：标记 TRIAGE_REQUIRED，执行 triage 后再继续。

**8. boundary 假设**
   列出路径上所有块声明的 `lifecycle_assumption=` 和
   `document_snapshot=` 值。
   说明每个假设在此次修改中是否成立。

**9. 运行时假设**
   列出路径上的 `caller_context=` 和 `runtime_guard=` 值。
   标记任何可能被此次修改违反的假设。

**10. 疑似不变量破坏**
    哪些规则或跨模块约束可能被此次修改违反？
    若无：明确写"未发现不变量破坏风险"。

**11. 验证计划**
    修改后检查什么？必须包含：
    - 运行哪些测试覆盖条目
    - 验证哪些链路日志序列
    - 更新哪些 triage 报告条目

### 6.3 Issue Trace 文档状态

Issue Trace 文档状态为 A。

Issue Trace 在执行期间约束任务范围（Scope Lock），任务完成后作为执行记录保留。
任务完成不改变文档状态，不得将已完成的 Issue Trace 改为 R 或 X。

### 6.4 文件范围控制

Issue Trace 完成后，必须产出文件范围结论，作为 ADP Scope Lock 的输入：

```
允许修改的文件：
  [列出文件]
  每个文件必须能从生产者块或消费者块中找到依据

禁止修改的文件：
  不在允许列表中的文件默认禁止
```

本节只定义 Issue Trace 的范围输出结构。
范围锁定、生效确认、范围扩展、阻断与恢复流程由 ADP 协议定义。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-05-18 | v1.0 | 完整重写，中文版，引入复合映射 ID 和结构化标注块 |
| 2026-05-19 | v1.1 | 规则生命周期简化（去除 accepted）；模块编码和规则域改为自动生成；链路 superseded 机制补全；手工维护区彻底移除；MODULE/DOMAIN 创建流程补充；triage VALID_EMPTY 边界清理；接入流程更新 |
| 2026-05-19 | v1.2 | MODULE 推断规则明确化；MODULE_AMBIGUOUS 用词修正（规则语义→模块职责归属）；CONSTRAINT 和 DOMAIN 扩展域创建流程补充；§1.2 阶段一完成标准补第4条 |
| 2026-05-19 | v1.3 | 编码格式规则更新（MODULE/DOMAIN 允许数字后缀）；删除扩展域重复描述行；constraint_id 补序号格式说明 |
| 2026-05-19 | v1.4 | 补充薄包装函数@GOV判断规则（§4.5）；补充covers注释格式规范和脚本扫描说明（§5.6）；修复测试覆盖字段工具链 |
| 2026-05-20 | v1.5 | 统一@GOV为块注释格式（唯一强制格式）；明确docs必须使用标注块格式；修复generate_adus.mjs行注释解析分支（回滚为只支持块注释）；timestamp churn修复；governance:audit与generate分离说明；接入前提要求补充 |
| 2026-05-20 | v1.6 | 补充 ADP 引用关系；Issue Trace §6 标注为 ADP TRACE_GATE 的上游定义；§4/@GOV 标注为 GOVERNANCE_UPDATE_GATE 依据；§5/triage 标注为 SC-03/SC-04 依据；文档头补充下游协议声明 |
| 2026-05-20 | v1.7 | 明确四份治理控制文档体系；移除非标准文档头字段；统一当前项目治理文件目录与 generate_adus 路径；将 Issue Trace 文件范围控制降级为 ADP Scope Lock 输入结构 |
| 2026-05-20 | v1.8 | 迁移治理控制文档到项目根目录；同步 generate_adus 路径为 scripts/generate_adus.mjs，输出根目录 ADUS.md |
| 2026-05-20 | v1.9 | §4.2/§4.3 补充 IO 和 QUERY 两种代码块类型，更新类型枚举和决策树；§6.3 措辞修正，移除对 ADP 下游概念的直接命名 |
| 2026-05-21 | v1.10 | 前言/名词定义/§2.1 L0：将治理控制文档从四份补全为五份，增加 APC；明确 APC 和 ADUS 为项目级，路径由 governance.config.json 声明 |
| 2026-05-21 | v1.11 | §3.2 新增 TERM 标注块格式、创建流程、三类 audit 检测类型；§4.2 新增 term_ref 可选字段；§4.6 新增 BOUNDARY_ABSTRACT 死线规则；§5.7 新增 AI 语义审计流程和输出格式；§5.8 新增 new_domain_concept_candidate 说明 |
| 2026-05-21 | v1.12 | §0.2 接入前提补充 TERM 可选标注块说明；步骤二补充 TERM 注册步骤（步骤4，原步骤4→5）；§4.6 BOUNDARY_ABSTRACT 词汇表补充 param / info |
| 2026-05-22 | v1.13 | §2.3 A 定义去除「当前」时间性限定，R 定义去除「包含草稿」；新增 §6.3 Issue Trace 文档状态规则（原 §6.3 顺延为 §6.4） |
| 2026-05-22 | v1.14 | §5.4 删除悬空句「豁免列表在 CLAUDE.md 中声明」；§0.2 脚本调用改为 `npm run governance:generate`；§3.2 TERM_ORPHAN 新增豁免说明（历史代码/未标注块不纳入统计）；§4.2 codes 字段允许 generated 状态；§4.4 映射 ID 生成时机说明同步；§4.5 新增 primary owner 四步判定层级 |
| 2026-05-22 | v1.15 | §4.6 新增 DELEGATE_ABSTRACT 规则（delegate= 必须是实际下游标识符，禁止架构描述词）；§4.5 新增链路入口强制标注规则 CHAIN_ENTRY_UNANNOTATED（ADUS 链路入口函数必须有 @GOV chain= 引用）；§5.2 新增第八种 triage 分类 BLOCK_UNANNOTATED（检测范围、豁免条件和 audit 输出规则） |
| 2026-05-22 | v1.16 | §5.1 新增 audit 正空间/负空间双扫描模式定义（两者均为必须执行项）；§5.7 补充语义审计触发点（VALIDATING 阶段必须消费 semantic_review_candidates，不得静默跳过）；§5.6 新增链路集成测试声明规范（integration_test 字段必填，deferred 为合法推迟声明，缺失报 CHAIN_INTEGRATION_TEST_MISSING） |
| 2026-05-22 | v1.17 | §1.2 补充 APC 可定义阶段一子阶段门禁；新增 §2.6 APC 项目覆盖机制；§3.2 新增 REQ、REQ_MAP 和可选 STATE_MACHINE 标注块及 ADUS 协同说明 |
| 2026-05-23 | v1.18 | §1.2 阶段二完成标准：「扫描脚本无 P0/P1 违规」改为「audit 无 CHAIN_ENTRY_UNANNOTATED error 或其他 error 级违规」（P0/P1 audit 分级未在 ADU 定义） |
| 2026-06-09 | v1.19 | §3.2 TERM_DUPLICATE 触发条件补充 term_id 字面冲突（同一 term_id 注册多次）；§4.2 term_ref 字段支持逗号分隔多 TERM ID，audit 逐个校验（同步引擎能力） |
| 2026-06-09 | v1.20 | 自审计修正：§3.2 CHAIN status 枚举补全 planned；§3.2 RULE 示例移除手写 registry_status 并加派生说明；§3.2 CHAIN 新增可选 risk_tier 字段，§5.5 日志分级由「P0/P1 链路」改为引用 risk_tier；§3.2 REQ_MAP、§4.6 DELEGATE_ABSTRACT、§5.6 CHAIN_INTEGRATION_TEST_MISSING 标注 enforcement: deferred（规范声明对齐引擎现状） |
| 2026-08-01 | v1.21 | TEST-GOVERNANCE-01：§0 接入 test_governance_path；§5.6 建立测试来源、模块 × 层级 × 风险模型、动态资产发现、变更影响选择、失败关闭与测试/验证分证据规则 |
| 2026-06-09 | v1.21 | 结构性减负：§4.3 新增「@GOV 强制标注范围」分层（强制=CHAIN 入口 error / 应当=业务可执行块 BLOCK_UNANNOTATED warning / 不要求=纯展示组件、薄包装、纯类型），明确人类可显式豁免须留痕；§4.5 CHAIN_ENTRY_UNANNOTATED 改述为引擎已实现的覆盖式检测、入口函数粒度标 deferred，superseded 链路加入豁免 |
