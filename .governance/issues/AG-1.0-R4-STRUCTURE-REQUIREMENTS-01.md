# Issue Trace — AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：文件树结构逻辑状态与显式保存

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

现有文件树工具把创建文件夹、重命名和移动直接写入磁盘，而文档编辑与新建已确认“先进入逻辑状态、接受不写盘、显式保存才写盘”。两种修改路径不一致，会使用户无法判断何时真正改变了项目，也无法在结构调整后安全处理打开标签、引用和待审阅内容提案。

## 2. 本轮人类确认

1. `CAP-11` 作为 P1 只纳入：由 Agent 或用户在文件树发起的创建文件夹、单个已保存文件重命名、单个已保存文件移动。
2. 三类操作先形成结构 Proposal，并投影到逻辑文件树与显示状态；接受只改变结构卡生命周期，不写磁盘。
3. 用户逐项显式保存文件树更改时，才重新核验并写入磁盘文件树；不承诺批量提交原子性或自动回滚。
4. 保存失败或冲突时，保持暂存结构并诚实说明；不得覆盖冲突目标或伪报已保存。
5. 受影响已打开标签保留内容并显示暂定路径；关联待审阅内容 Proposal 保守失效。
6. 复制、删除、目录移动、批量提交和格式转换不在本子项；删除和格式转换分别留待后续 R4 子项。

## 3. 需求边界

- 文件树结构 Proposal 不等同于 DE 的内容 ChangeProposal/Diff；DE 不因本轮承担结构卡或内容载荷。
- 逻辑文件树是工作区结构的逻辑状态，不代表已落盘目录；具体对象、字段、状态机、保存入口和 UI 归后续 DESIGN Scope。
- 已发送消息中的引用保持发送时冻结快照；新回合不得把结构变更前的路径伪装成当前路径。
- 本轮不改变文档内容的 LogicalState 或 BDocx/格式转换语义。

## 4. 直接消费者

- CORE：REQ-CORE-012 作为产品级结构逻辑状态与显式保存根要求。
- AG：REQ-AG-011 只生成结构 Proposal，不再把整理请求直接交直写工具。
- WS：REQ-WS-008 承接逻辑文件树、逐项保存、基线核验和失败保留。
- ED：REQ-ED-013 承接已打开标签的暂定路径与内容/脏态保留。
- REF：REQ-REF-003 承接结构路径基线推进后的引用快照与失效规则。

## 5. 验收计划

- 建目录、单文件重命名、单文件移动均覆盖预览、接受、拒绝、显式保存、冲突/失败和外部变更。
- 预览和接受都不写入磁盘；逐项显式保存才改变磁盘文件树。
- 受影响标签保留内容/脏态并更新暂定路径；关联待审阅内容 Proposal 失效；引用不伪装旧路径为当前路径。
- 复制、删除、目录移动、批量提交和格式转换不得因本能力露出可执行入口。
- 本 Scope 不运行代码测试；现有直写工具、接口与实现迁移属于后续 DESIGN/IMPLEMENTATION Scope。

## 6. 待设计项

- 结构 Proposal、逻辑文件树、结构卡、保存入口、基线快照和重启恢复的对象/状态机/接口。
- 单项保存的冲突检测、外部变化、失败后同步及时间轴/撤销关系。
- 标签身份与路径分离、REF 暂定路径引用、内容 Proposal 失效事件与 Trace。
- `ws.create_folder`、`ws.rename_doc`、Agent `move_doc`/`create_folder` 现有直写实现的迁移、feature gate 和真实验证。

## 7. 文件范围结论

允许修改：CORE 产品根与需求索引、能力基线、AG/WS/ED/REF 的直接 L1 需求、本 Issue Trace 与 active scope。

禁止修改：DE、DOC、L2 技术设计/状态机/接口、代码、测试、脚本、APC、ADUS 和治理配置；它们不因本轮需求确认而直接变更，或必须等后续 DESIGN Scope 承接。

## 8. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：`A-CORE-C-D-01` 产品根、`A-CORE-X-D-01` 需求索引、`A-AG-M-D-02` 能力基线、AG/WS/ED/REF 四份直接 L1 需求文档、本 Issue Trace 和 active scope。
- scope：人类已明确确认 R4-B；active scope 只列出本轮直接消费者和治理留痕文件。
- phase_gate：G1 REQUIREMENT 进行中；未进入 DESIGN 或 IMPLEMENTATION。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT 或 TERM。
- test_impact：不适用；本轮无代码或测试改动。
- validation：已逐段复核 CAP-11、CORE/AG/WS/ED/REF 的逻辑文件树、接受不写盘、逐项显式保存、失败保留、标签暂定路径、引用快照与内容 Proposal 失效边界；Node 只读检查 active scope 的全部必需文件均在允许范围；CORE 12 条、AG 11 条、WS 8 条、ED 13 条、REF 7 条 REQ 均具备必填字段且 req_id 无重复；当前正文不含“整理请求可实际移动/建目录”或“可经 WS 边界工具直接执行”遗留；`git diff --check` 对已跟踪 Scope 文件无输出；包含未跟踪文档在内的尾随空白扫描无命中。
- automated_tests：不适用。文件树新基线尚未实现；本轮不将需求确认表述为代码或系统验证已通过。
- unresolved_items：见 §6；R4-C/R4-D、R5 与后续 DESIGN Scope 仍待完成。
