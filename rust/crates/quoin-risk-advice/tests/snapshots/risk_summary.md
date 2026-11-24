Regions come from the project's policy. Decisions come from the named person who made them.

| hazard | hazard_title | hazard_text | situation | situation_title | situation_text | harms | harm_texts | pre_control | controls | post_control | residual_risks | findings |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HAZ-1 | HAZ-1 | null | SIT-1 | SIT-1 | null | HARM-1 | harm=HARM-1; title=HARM-1; text=null | estimate=EST-A; severity=Serious; probability=Likely; the policy marks this cell as High | control=CTRL-1; requirements=FR-9; retired_requirements=[]; verifications=TC-9 | estimate=EST-B; severity=Minor; probability=Rare; the policy marks this cell as Low | residual_risk=RR-1; decision=accepted; decided_by=Pat; rationale=r | [] |
| HAZ-2 | HAZ-2 | null | SIT-2 | SIT-2 | null | [] | [] | [] | control=CTRL-2; requirements=[]; retired_requirements=[]; verifications=[] | estimate=EST-C; severity=Serious; probability=Likely; the policy marks this cell as High<br>estimate=EST-D; severity=Serious; probability=not_assessed; policy_region=null | residual_risk=RR-2; decision=accepted; decided_by=Pat; rationale=r<br>residual_risk=RR-3; decision=not_accepted; decided_by=Pat; rationale=r | finding=situation-without-harm; names=SIT-2<br>finding=control-without-requirement; names=CTRL-2<br>finding=control-without-verification; names=CTRL-2<br>finding=post-control-without-control; names=EST-D<br>finding=decision-without-region; names=RR-3<br>finding=benefit-risk-undecided; names=RR-2, BR-1 |
| HAZ-3 | HAZ-3 | null | null | null | null | [] | [] | [] | [] | [] | [] | finding=hazard-without-situation; names=HAZ-3 |
| HAZ-4 | HAZ-4 | null | SIT-4 | SIT-4 | null | HARM-4 | harm=HARM-4; title=HARM-4; text=null | [] | control=CTRL-1; requirements=FR-9; retired_requirements=[]; verifications=TC-9 | estimate=EST-E; severity=Serious; probability=Likely; the policy marks this cell as High<br>estimate=EST-F; severity=Serious; probability=Likely; the policy marks this cell as High<br>estimate=EST-G; severity=Minor; probability=Rare; the policy marks this cell as Low<br>estimate=EST-H; severity=Minor; probability=Rare; the policy marks this cell as Low<br>estimate=EST-I; severity=Minor; probability=Rare; policy_region=null | residual_risk=RR-4; decision=accepted; decided_by=Pat; rationale=r<br>residual_risk=RR-5; decision=accepted; decided_by=Pat; rationale=r<br>residual_risk=RR-6; decision=not_accepted; decided_by=Pat; rationale=r<br>residual_risk=RR-7; decision=undecided; decided_by=null; rationale=r | finding=post-control-estimate-without-decision; names=EST-I<br>finding=residual-risk-undecided; names=RR-7<br>finding=accepted-outside-acceptable-region; names=RR-4<br>finding=benefit-risk-not-accepted; names=RR-5, BR-2<br>finding=not-accepted-inside-acceptable-region; names=RR-6 |
| HAZ-5 | Overdose | The pump delivers<br>too much. | null | null | null | [] | [] | [] | [] | [] | [] | finding=hazard-without-situation; names=HAZ-5 |
| null | null | null | SIT-3 | SIT-3 | null | [] | [] | [] | [] | [] | [] | finding=situation-without-hazard; names=SIT-3<br>finding=situation-without-harm; names=SIT-3 |

| benefit_risk | benefit | decision | decided_by | rationale | residual_risks |
| --- | --- | --- | --- | --- | --- |
| BR-1 | b | undecided | null | r | RR-2 |
| BR-2 | b | not_accepted | Pat | r | RR-5 |

| evaluation | product | decision | decided_by | rationale | references |
| --- | --- | --- | --- | --- | --- |
| ORE-1 | Pump | undecided | null | pending | BR-2, RR-1 |
