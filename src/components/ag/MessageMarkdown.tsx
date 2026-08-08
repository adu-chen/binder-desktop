import { memo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import type { Components } from "react-markdown";

/**
 * @GOV
 * codes:    BR-AG-UI-001-RB-AG-AG-TURN-012
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-UI-001
 * term_ref: TERM-AG-011
 * boundary: in=AssistantMessageText | out=sanitizedMarkdownElements | effect=memory | caller_context=messageFlowRender | does_not=execute_raw_HTML_or_allow_webview_navigation
 */
export const MessageMarkdown = memo(function MessageMarkdown({ content }: { content: string }) {
  return (
    <div className="msg-md">
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[[rehypeSanitize, schema]]} components={components}>
        {content}
      </ReactMarkdown>
    </div>
  );
});

/** sanitize 白名单（rehype-sanitize 默认 schema + 保留语言类名；GFM 表格标签默认已含）。 */
const schema = {
  ...defaultSchema,
  attributes: {
    ...defaultSchema.attributes,
    "*": [...(defaultSchema.attributes?.["*"] ?? []), "className"],
  },
};

/** 链接仅允许 http/https，并阻止 Tauri WebView 内导航（BR-AG-UI-001）。 */
function SafeLink({ href, children, ...props }: { href?: string; children?: React.ReactNode } & React.ComponentPropsWithoutRef<"a">) {
  const safe = typeof href === "string" && /^https?:\/\//i.test(href);
  return (
    <a
      {...props}
      href={safe ? href : undefined}
      rel="noopener noreferrer"
      title={safe ? href : undefined}
      onClick={(event) => event.preventDefault()}
    >
      {children}
    </a>
  );
}

const components: Components = {
  a: SafeLink,
  pre({ children }) {
    return <pre className="msg-pre">{children}</pre>;
  },
  code({ className, children, ...props }) {
    const match = /language-([\w-]+)/.exec(className ?? "");
    if (match) {
      return (
        <code className={className} {...props}>
          <span className="msg-code-lang">{match[1]}</span>
          {children}
        </code>
      );
    }
    return (
      <code className={className} {...props}>
        {children}
      </code>
    );
  },
};
