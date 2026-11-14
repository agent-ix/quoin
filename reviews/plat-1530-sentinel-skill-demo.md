# PLAT-1530 positive sentinel skill demonstration

## Scope

This run followed the local skills/spec-security-analysis/SKILL.md process using only a synthetic pack. The pack's clause titles and text contained a unique synthetic sentinel. No restricted standard text was used.

## Inputs and executable

- PR source head: 14e6a21b421f72bf7208d7661595312eb61b50fc.
- PR-head executable: /home/peter/dev/worktrees/plat-1530-quoin/rust/target/debug/quoin, version 0.31.0-48-g14e6a21b-dirty.
- Isolated scratch repo and IX home: /var/tmp/codex-controls-1530-sentinel-demo/repo and /var/tmp/codex-controls-1530-sentinel-demo/ix-home.
- Synthetic pack: /var/tmp/codex-controls-1530-sentinel-demo/pack-src; exact key example.invalid/std@1; IDs EX-1 and EX-2. The sentinel was present only in the pack's clause title/text.

## Results

- quoin module install exited 0 from the /var/tmp pack path; module-list.txt reports the installed module as pack=true.
- The repo profile adds exactly example.invalid/std@1. Profile evaluation reports that set as evaluated, with EX-1 and EX-2 both binding. The unavailable NIST profile set was not mapped.
- quoin write --types control,SpecReview --json returned the installed control and SpecReview skeleton paths.
- The authored control maps only example.invalid/std@1 / EX-1 to FR-002.
- The authored SpecReview records a synthetic gap for example.invalid/std@1 / EX-2.
- The reply reports those set keys and clause IDs only.
- quire validate --scope <scratch-repo> "spec/security/*.md" "reviews/SR-001-gap-analysis.md" exited 0. It emitted only the previously observed local-source-identity and unchecked-OCL warnings plus the unused imported module schema warning.
- A literal exact-match scan over the control artifact, SpecReview, reply, profile evaluation, module listing, write pack and validation output found no sentinel. The sentinel remained only in the installed synthetic pack source.

## Evidence paths

Generated outputs are in /var/tmp/codex-controls-1530-sentinel-demo/: pack-src/std.yaml, pack-install.txt, module-list.txt, profile-evaluation.txt, write-pack.json, repo/spec/security/CTRL-001-synthetic-authorization.md, repo/reviews/SR-001-gap-analysis.md, repo/demo-reply.md, and quire-validate-sentinel.txt.
