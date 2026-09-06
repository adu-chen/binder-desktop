import { Children, type ReactNode } from "react";
import type { ProjectReportView } from "../../store/agStore";

/**
 * @GOV
 * codes:    BR-AG-OBS-001-RB-AG-AG-TURN-050, BR-AG-UI-001-RB-AG-AG-TURN-050
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-OBS-001, BR-AG-UI-001
 * term_ref: TERM-AG-021, TERM-AG-023
 * boundary: in=RuntimeDerivedProjectReportForAssistantTurn | out=ReadUnreadFindingAndUncertaintyReadOnlyCard | effect=none | does_not=inferCoverageFromMessageToolCardOrUIState
 *
 * ProjectReport（TERM-AG-021）只读呈现：内容完全来自运行时基于 EvidenceLedger 派生的安全事件，
 * 不从 assistant 正文、工具卡状态或界面缓存推断读取覆盖与确定性。
 */
export function ProjectReportCard({ report }: { report: ProjectReportView }) {
  return (
    <section className="project-report" aria-label="项目报告">
      <div className="project-report-head">
        <span className="project-report-title">项目报告</span>
        <span className="project-report-cursor">证据截至 #{report.evidenceThroughSeq}</span>
      </div>

      <ReportSection title={`已读 ${report.readItems.length}`} empty="没有确认已读的文档">
        {report.readItems.map((item) => (
          <li key={`${item.relPath}:${item.source}:${item.coverage}`}>
            <span>{item.relPath}</span>
            <span className="project-report-meta">{item.coverage} · {item.source}</span>
          </li>
        ))}
      </ReportSection>

      <ReportSection title={`未读 ${report.unreadItems.length}`} empty="请求范围内没有未读项">
        {report.unreadItems.map((item) => (
          <li key={`${item.relPath}:${item.reason}`}>
            <span>{item.relPath}</span>
            <span className="project-report-meta">{item.reason}</span>
          </li>
        ))}
      </ReportSection>

      {report.findings.length > 0 && (
        <ReportSection title={`发现 ${report.findings.length}`}>
          {report.findings.map((finding, index) => (
            <li key={`${index}:${finding.statement}`}>
              <span>{finding.statement}</span>
              <span className="project-report-meta">不确定性：{finding.uncertainty}</span>
            </li>
          ))}
        </ReportSection>
      )}

      {report.uncertainties.length > 0 && (
        <ReportSection title={`仍不确定 ${report.uncertainties.length}`}>
          {report.uncertainties.map((item, index) => <li key={`${index}:${item}`}>{item}</li>)}
        </ReportSection>
      )}
    </section>
  );
}

function ReportSection({
  title,
  empty,
  children,
}: {
  title: string;
  empty?: string;
  children?: ReactNode;
}) {
  return (
    <div className="project-report-section">
      <span className="project-report-section-title">{title}</span>
      <ul>{Children.count(children) > 0 ? children : <li className="project-report-empty">{empty}</li>}</ul>
    </div>
  );
}
