/**
 * 分页流只描述已经测得的视觉片段，不持有 DOM 或 ProseMirror Node。
 * `pos` 是当前文档内可映射的位置；`height` 是扣除既有分页缝后的未缩放 CSS px。
 */
export type PageBreakKind = "block" | "paragraph-line";

export interface RenderedFlowFragment {
  pos: number;
  height: number;
  kind: PageBreakKind;
  blockPos: number;
  lineIndex: number;
  lineCount: number;
}

export interface RenderedFlowBlock {
  blockPos: number;
  kind: "atomic" | "paragraph";
  fragments: RenderedFlowFragment[];
}

export interface PageBreak {
  /** 同一方案内按页序稳定生成；DOM 使用 id，不能把会随事务映射的 pos 写入 dataset。 */
  id: string;
  pos: number;
  kind: PageBreakKind;
  fill: number;
}

export interface PageBreakPlan {
  breaks: PageBreak[];
  lastFill: number;
}

export interface PaginationFlowPlanner {
  /** 当前页是否无法完整容纳该块；用于只对实际需要拆分的普通段落启动行盒测量。 */
  wouldOverflow(blockHeight: number): boolean;
  append(block: RenderedFlowBlock): void;
  finish(): PageBreakPlan;
}

type PendingBreak = Omit<PageBreak, "id">;

function fragmentHeight(fragment: RenderedFlowFragment): number {
  return Number.isFinite(fragment.height) ? Math.max(0, fragment.height) : 0;
}

function blockHeight(block: RenderedFlowBlock): number {
  return block.fragments.reduce((sum, fragment) => sum + fragmentHeight(fragment), 0);
}

function breakKind(fragment: RenderedFlowFragment): PageBreakKind {
  return fragment.kind === "paragraph-line" ? "paragraph-line" : "block";
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-RB-ED-ED-EDIT-017
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=RenderedFlowBlockSequenceAndPageContentHeight | out=PageBreakPlan(proseMirrorPositionBreaksWithPageFill)
 */
/// 分页流规划守则（BR-ED-DATA-003）：只消费已测量的视觉片段和当前文档 position，输出纯呈现分页计划；
/// 不读取或改写文档模型。原子 Block 延续既有整块分页；普通段落采用贪心填页，当前页保留所有能够完整
/// 容纳的视觉行，只有一行都放不下时才换页；超整页行以“必须前进”为准接受溢出。
export function createPaginationFlowPlanner(budget: number): PaginationFlowPlanner {
  const validBudget = Number.isFinite(budget) && budget > 0 ? budget : 0;
  const breaks: PendingBreak[] = [];
  let used = 0;
  let finished: PageBreakPlan | null = null;

  const addBreak = (fragment: RenderedFlowFragment) => {
    if (used <= 0) return;
    breaks.push({
      pos: fragment.pos,
      kind: breakKind(fragment),
      fill: Math.max(0, validBudget - used),
    });
    used = 0;
  };

  const appendAtomic = (block: RenderedFlowBlock) => {
    const height = blockHeight(block);
    const first = block.fragments[0];
    if (!first) return;
    if (used > 0 && used + height > validBudget) addBreak(first);
    used += height;
  };

  const prefixThatFits = (
    fragments: RenderedFlowFragment[],
    from: number,
    available: number,
  ): number => {
    let height = 0;
    let count = 0;
    for (let i = from; i < fragments.length; i++) {
      const next = fragmentHeight(fragments[i]);
      if (count > 0 && height + next > available) break;
      if (count === 0 && next > available) return 0;
      height += next;
      count += 1;
    }
    return count;
  };

  const appendParagraph = (block: RenderedFlowBlock) => {
    const fragments = block.fragments;
    if (fragments.length < 2) {
      appendAtomic(block);
      return;
    }

    let cursor = 0;
    while (cursor < fragments.length) {
      const available = Math.max(0, validBudget - used);
      const fit = prefixThatFits(fragments, cursor, available);

      // 当前页连一行都放不下才换页；能放一行就保留一行，不为防孤行主动制造页尾空白。
      if (used > 0 && fit === 0) {
        addBreak(fragments[cursor]);
        continue;
      }

      if (fit === 0) {
        // 空白页仍容不下第一行：接受该超高行，确保算法和编辑均能继续前进。
        used += fragmentHeight(fragments[cursor]);
        cursor += 1;
      } else {
        for (let i = 0; i < fit; i++) used += fragmentHeight(fragments[cursor + i]);
        cursor += fit;
      }

      if (cursor < fragments.length) addBreak(fragments[cursor]);
    }
  };

  return {
    wouldOverflow(height) {
      if (validBudget <= 0) return false;
      const normalized = Number.isFinite(height) ? Math.max(0, height) : 0;
      return normalized > Math.max(0, validBudget - used);
    },
    append(block) {
      if (finished || validBudget <= 0 || block.fragments.length === 0) return;
      if (block.kind === "paragraph") appendParagraph(block);
      else appendAtomic(block);
    },
    finish() {
      if (finished) return finished;
      finished = {
        breaks: breaks.map((pageBreak, index) => ({
          ...pageBreak,
          id: `pg-${index + 2}`,
        })),
        lastFill: validBudget > 0 ? Math.max(0, validBudget - used) : 0,
      };
      return finished;
    },
  };
}
