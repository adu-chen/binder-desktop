import type { TaskPlanView } from "../../store/agStore";

/**
 * PlanCard（TERM-AG-018 · AG-1.0-TASKRUN-REWORK-CONTINUE-01）。
 * 任务计划呈现：可展示操作计划（非思维链）——步骤列表、依赖、当前游标、确认点标记。
 * 只读呈现；真实执行进度由运行事件驱动。
 */

const STEP_KIND_LABEL: Record<string, string> = {
  read: "读取",
  tool: "工具",
  analyze: "分析",
  generate: "生成",
  propose: "提案",
  confirm: "确认",
};

export function PlanCard({ plan }: { plan: TaskPlanView }) {
  return (
    <div className="plancard">
      <div className="plancard-head">
        <span className="plancard-title">任务计划</span>
        <span className="plancard-cursor">游标 {plan.cursor}/{plan.steps.length}</span>
      </div>
      <ol className="plancard-steps">
        {plan.steps.map((step) => {
          const done = step.status === "ok" || step.status === "failed" || step.status === "cancelled";
          const active = step.seq === plan.cursor - 1 && !done;
          const deps = step.deps.length ? `依赖 ${step.deps.join(",")}` : "";
          return (
            <li key={step.stepId} className={`planstep planstep-${step.status}${active ? " planstep-active" : ""}`}>
              <span className="planstep-kind">{STEP_KIND_LABEL[step.kind] ?? step.kind}</span>
              <span className="planstep-label">{step.label}</span>
              {step.confirmPoint && <span className="planstep-confirm">确认点</span>}
              {step.requiredGrant && <span className="planstep-grant">{step.requiredGrant}</span>}
              {deps && <span className="planstep-deps">{deps}</span>}
            </li>
          );
        })}
      </ol>
    </div>
  );
}
