/**
 * Rendering an assurance case (FR-040).
 *
 * Deterministic markdown plus a mermaid graph. **A store view, never a
 * hand-maintained document** — every re-render over unchanged inputs produces
 * byte-identical output, so a diff is a change in the evidence and not in
 * somebody's prose.
 */

import type { AssuranceCase, CaseNode } from "./graph.js";

/** GSN marks an unsupported goal as undeveloped; these are the two states. */
const MARK: Record<CaseNode["status"], string> = {
  supported: "✅",
  open: "◇",
};

export function renderCase(assurance: AssuranceCase): string {
  const lines: string[] = ["# Assurance case", ""];

  if (assurance.claims.length === 0) {
    // Not an empty case — no case. The distinction matters: a reader who sees
    // an empty document concludes there is nothing to argue, and the truth is
    // that nothing declared itself a claim.
    lines.push(
      "No document declares itself a top-level claim, so there is no case to",
      "argue. Declare an `StR` (or pass `--claim-type`) and re-run.",
      "",
    );
  }

  const open = assurance.claims.filter((c) => c.status === "open").length;
  if (assurance.claims.length > 0) {
    lines.push(
      `${assurance.claims.length} claim(s), ${open} with an open branch.`,
      "",
    );
  }

  for (const claim of assurance.claims) {
    lines.push(`## ${MARK[claim.status]} ${claim.id} — ${claim.statement}`, "");
    lines.push(...renderNode(claim, 0));
    lines.push("");
    lines.push("```mermaid", ...mermaidFor(claim), "```", "");
  }

  if (assurance.unreachable.length > 0) {
    lines.push(
      "## Reachable from no claim",
      "",
      "These carry obligations and no claim argues over them. A case rendered",
      "only over the reachable half would read as complete.",
      "",
      ...assurance.unreachable.map((id) => `- ${id}`),
      "",
    );
  }

  if (assurance.unreadable.length > 0) {
    lines.push(
      "## Unreadable",
      "",
      ...assurance.unreadable.map((u) => `- ${u.path}: ${u.reason}`),
      "",
    );
  }

  return lines.join("\n");
}

function renderNode(node: CaseNode, depth: number): string[] {
  const indent = "  ".repeat(depth);
  const label =
    node.kind === "solution"
      ? `${indent}- ${MARK[node.status]} **${node.id}** — ${node.statement}`
      : `${indent}- ${MARK[node.status]} ${node.id} — ${node.statement}`;
  const out = depth === 0 ? [] : [label];
  if (node.because) out.push(`${indent}  - ↳ ${node.because}`);
  for (const child of node.children) {
    out.push(...renderNode(child, depth + 1));
  }
  return out;
}

/**
 * The claim's subtree as a mermaid flowchart.
 *
 * Three authoring rules this obeys, each of which produces a diagram that fails
 * to render rather than one that renders wrongly:
 *
 * - **Ids are sanitised.** `FR-001-AC-1` contains `-`, and mermaid reads a bare
 *   hyphenated token as an edge fragment.
 * - **Labels are quoted**, because a statement containing `(` or `)` ends the
 *   node shape early.
 * - **No `;` in label text** — it terminates the statement.
 */
function mermaidFor(claim: CaseNode): string[] {
  const lines = ["flowchart TD"];
  const seen = new Set<string>();
  const walk = (node: CaseNode): void => {
    const id = nodeId(node.id);
    if (!seen.has(id)) {
      seen.add(id);
      const shape = node.kind === "solution" ? ["([", "])"] : ["[", "]"];
      lines.push(`  ${id}${shape[0]}"${label(node)}"${shape[1]}`);
    }
    for (const child of node.children) {
      walk(child);
      lines.push(`  ${id} --> ${nodeId(child.id)}`);
    }
  };
  walk(claim);
  return lines;
}

function nodeId(id: string): string {
  return id.replace(/[^A-Za-z0-9]/g, "_");
}

function label(node: CaseNode): string {
  const text = `${node.id}: ${node.statement}`;
  return text
    .replace(/["`]/g, "'")
    .replace(/;/g, ",")
    .slice(0, 80)
    .concat(node.status === "open" ? " ◇" : "");
}
