import type { TaskPlanView } from "../../store/agStore";

/**
 * PlanCard（TERM-AG-018 · AG-1.0-TASKRUN-REWORK-CONTINUE-01）。
 * 当前候选路线的轻量投影。candidate 不承诺执行，也不显示授权或执行游标。
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
        <span className="plancard-title">当前路线</span>
        <span className="plancard-cursor">修订 {plan.routeRevision}</span>
      </div>
      <div className="plancard-summary">{plan.summary}</div>
      <ol className="plancard-steps">
        {plan.candidateSteps.map((step) => {
          const deps = step.dependsOn.length ? `依赖 ${step.dependsOn.join(",")}` : "";
          return (
            <li key={step.stepId} className={`planstep planstep-${step.status}`}>
              <span className="planstep-kind">{STEP_KIND_LABEL[step.kind] ?? step.kind}</span>
              <span className="planstep-label">{step.label}</span>
              {deps && <span className="planstep-deps">{deps}</span>}
            </li>
          );
        })}
      </ol>
    </div>
  );
}
