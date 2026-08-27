const SECTION_HIT_RATE = "minting.section_hit_rate";
const MARK = {
  improved: "++",
  held: "ok",
  new: "**",
  regressed: "!!",
  incomparable: "??",
};

/** Render a Tier-1 report and its ratchet verdicts from the same report object. */
export function renderTier1(report, verdicts) {
  const pct = (value) => {
    if (value === null) return "  n/a";
    const percent = value * 100;
    const shown =
      percent !== 0 && Math.abs(percent) < 1
        ? percent.toFixed(1)
        : String(Math.round(percent));
    return `${shown}%`.padStart(5);
  };
  const declaration = report.provenance.declaration;
  const lines = [
    `tier-1: ${report.corpora} corpora, ${report.findings} findings mapped` +
      ` (${report.by_language
        .map((row) => `${row.language} ${row.corpora}`)
        .join(", ")})`,
    `engine      ${report.provenance.engine}`,
    `corpus      ${short(report.provenance.corpus)}`,
    `corpus input ${short(report.provenance.corpus_input)}`,
    `declaration ${declaration.root} ${short(declaration.digest)}` +
      (declaration.sources
        ? ` (${Object.entries(declaration.sources)
            .map(([name, sha]) => `${name} ${short(sha)}`)
            .join(", ")})`
        : " (no VENDORED.md: no upstream SHA recorded)"),
    `corpus gaps ${report.bounds?.gap_count ?? "not_computed"} of ${report.bounds?.declared_cells ?? "?"} declared mode-language cells`,
    "",
    "family                     TP  FP  miss   prec  recall",
  ];
  for (const family of report.families) lines.push(familyRow(family, "  "));
  lines.push(
    "",
    `finding_localisation_rate  ${pct(report.finding_localisation_rate)} ` +
      `(${report.positional} of ${report.families.reduce((sum, family) => sum + family.truePositives, 0)} true positives named where)`,
    `actionability_v1_rate      ${pct((report.actionability_v1 ?? report.actionability).rate)} ` +
      `(${(report.actionability_v1 ?? report.actionability).actionable} of ${(report.actionability_v1 ?? report.actionability).total} findings name a row or a line)`,
    `actionability_v2_rate      ${pct(report.actionability_v2?.rate ?? null)} ` +
      `(${report.actionability_v2?.numerator ?? 0} of ${report.actionability_v2?.denominator ?? 0} findings carry subject/locus, evidence, target, and next move; ` +
      `${report.actionability_v2?.exclusions?.length ?? 0} excluded)`,
    `minting.section_hit_rate   ${
      report[SECTION_HIT_RATE] === null
        ? "  n/a (no case reports it — this engine predates quire-rs#270)"
        : `${pct(report[SECTION_HIT_RATE].rate)} (${report[SECTION_HIT_RATE].matched}` +
          ` of ${report[SECTION_HIT_RATE].examined} declared minting documents` +
          ` reached their section, over ${report[SECTION_HIT_RATE].cases_reporting} cases)`
    }`,
    `cost_per_confirmed_insight ${String(
      report.cost_per_confirmed_insight?.toolCallsPer ?? "n/a",
    ).padStart(5)} tool calls per true positive (${
      report.cost_per_confirmed_insight?.toolCalls ?? "n/a"
    } calls, ${report.cost_per_confirmed_insight?.truePositives ?? 0} confirmed;` +
      " tokens n/a — tier 1 calls no model)",
    `sentinel.silent_zero       ${String(
      report["sentinel.silent_zero"]?.count ?? 0,
    ).padStart(5)} (GATE: ratio-shaped metrics reading none of a non-zero` +
      " population, with nothing saying so)",
  );
  lines.push("", "detection.recall (every row carries the corpus GAP count):");
  for (const row of report.detection_recall ?? []) {
    lines.push(
      `  ${row.mode.padEnd(12)} ${row.language.padEnd(10)} ${row.level} ` +
        `${pct(row.rate)} (${row.reached}/${row.population}; GAP ${row.gap_count})` +
        (row.misses.length ? ` — missed ${row.misses.join(", ")}` : ""),
    );
  }
  for (const instance of report["sentinel.silent_zero"]?.instances ?? []) {
    lines.push(
      `  ${instance.corpus}: ${instance.metric} walked ${instance.examined} and matched none,` +
        ` over ${instance.population}, unaccompanied`,
    );
  }
  const unread = report["sentinel.silent_zero"]?.unread_population ?? [];
  if (unread.length) {
    lines.push(
      `  reported, not gated — ${unread.length} ratio${unread.length === 1 ? "" : "s"}` +
        " over a population the engine walked NONE of, and said nothing about" +
        " (quire-rs#271): " +
        unread.map((item) => `${item.corpus}/${item.metric}`).join(", "),
    );
  }
  lines.push("", "by language:");
  for (const language of report.by_language) {
    lines.push(`  ${language.language} (${language.corpora} corpora)`);
    for (const family of language.families) {
      lines.push(familyRow(family, "    "));
    }
    if (!language.families.length) {
      lines.push("    (no family scored a finding or a label here)");
    }
  }
  if (report.collateral.length) {
    lines.push(
      "",
      "declared collateral, set aside from scoring:",
      ...report.collateral.map((item) => `  ${item.family} (${item.reason})`),
    );
  }
  if (report.excluded.length) {
    lines.push(`excluded as not findable: ${report.excluded.join(", ")}`);
  }
  lines.push("", "ratchet:");
  const wholeRun = verdicts.every(
    (verdict) => verdict.verdict === "incomparable",
  );
  for (const verdict of verdicts) {
    const dimensions = verdict.mode
      ? `[${verdict.level}/${verdict.mode}/${verdict.language}]`
      : "";
    const name = verdict.family
      ? `${verdict.metric}[${verdict.family}]`
      : `${verdict.metric}${dimensions}`;
    const why =
      wholeRun && verdict.verdict === "incomparable" ? null : verdict.why;
    lines.push(
      `  ${MARK[verdict.verdict]} ${name.padEnd(40)} ${verdict.observed} (baseline ${verdict.baseline})` +
        (why
          ? ` — ${why}`
          : verdict.verdict === "incomparable"
            ? " — not compared"
            : ""),
    );
  }
  if (verdicts.some((verdict) => verdict.verdict === "incomparable")) {
    lines.push(
      "",
      "INCOMPARABLE — this run and the baseline are over unlike inputs, so NO",
      "DELTA WAS COMPUTED. Both values are printed above; neither `improved`",
      "nor `regressed` is claimed, because a change was not observed — the",
      "subject changed.",
      `  ${verdicts.find((verdict) => verdict.verdict === "incomparable").why}`,
      "Re-baseline deliberately with `make bench-tier1-update` once the new",
      "inputs are the ones the gate should track.",
    );
  } else if (verdicts.some((verdict) => verdict.verdict === "regressed")) {
    lines.push(
      "",
      "REGRESSED — the baseline is kept, never lowered by a bad run.",
    );
  } else if (verdicts.some((verdict) => verdict.verdict === "improved")) {
    lines.push(
      "",
      "improved — run `make bench-tier1-update` to move the baseline.",
    );
  }
  return lines.join("\n");
}

function familyRow(family, indent) {
  const pct = (value) =>
    value === null ? "  n/a" : `${Math.round(value * 100)}%`.padStart(5);
  const row =
    `${indent}${family.family.padEnd(24)} ${String(family.truePositives).padStart(2)}  ` +
    `${String(family.falsePositives).padStart(2)}  ${String(family.misses).padStart(4)}  ` +
    `${pct(family.precision)}  ${pct(family.recall)}`;
  const basis = family.precision_basis;
  if (!basis) return row;
  const ruled = basis.truePositives + basis.falsePositives;
  return (
    `${row}   advisory: prec over ${ruled} ruled firing${ruled === 1 ? "" : "s"}` +
    (basis.byStanding
      ? ` (${basis.byStanding} by a standing ruling, ${ruled - basis.byStanding} per case)`
      : "") +
    `, ${basis.unadjudicated} UNADJUDICATED`
  );
}

function short(value) {
  if (typeof value === "string" && value.startsWith("sha256:")) {
    return `sha256:${value.slice(7, 19)}…`;
  }
  if (typeof value === "string" && /^[0-9a-f]{40}$/.test(value)) {
    return `${value.slice(0, 12)}…`;
  }
  return typeof value === "object" && value !== null
    ? JSON.stringify(value)
    : String(value);
}
