# Establishment Evaluation — lead-a-pilot-01

Adjudicate only against the frozen criteria in `docs/product/v0.2-lead-a-assisted-pilot-protocol.md`.

Do not pre-fill PASS. Use `PASS`, `FAIL`, or `UNEVALUATED` only.

## Metadata

- pilot_id: `lead-a-pilot-01`
- adjudicated_at: null
- adjudicator: null
- protocol_ref: `docs/product/v0.2-lead-a-assisted-pilot-protocol.md`
- versioning_authority: `docs/product/versioning.md`

## Frozen Criteria

```yaml
qualifying_participant: UNEVALUATED
real_authorized_material: UNEVALUATED
participant_operated_workflow: UNEVALUATED

workflow_completion:
  session_created_or_opened: UNEVALUATED
  review_cases_understood_enough_to_act: UNEVALUATED
  all_cases_dispositioned: UNEVALUATED
  reviewed_projection_reached: UNEVALUATED
  export_completed: UNEVALUATED

output_legibility:
  participant_can_identify_reviewed_output: UNEVALUATED
  participant_understands_that_machine_findings_are_not_authority: UNEVALUATED
  participant_understands_their_decisions_control_reviewed_output: UNEVALUATED

blocking_product_failure: none
```

## Overall Pilot Verdict

```text
pilot_verdict: UNEVALUATED
v0_2_establishment_recommendation: UNEVALUATED
```

`v0_2_establishment_recommendation` may be `recommend_establish`, `recommend_do_not_establish`, or `UNEVALUATED`. Owner establishment decision remains separate.

## Evidence Links

- participant-profile.json:
- input-manifest.json:
- session-observation.json:
- facilitator-log.md:
- participant-response.md:
- export-manifest.json:

## Adjudication Notes

Record only facts tied to criteria above. Do not import out-of-scope success conditions (liking the product, time saved, reuse value, ASR accuracy, payment intent).

## Negative Evidence Retention

If any core criterion is FAIL, retain this packet unchanged and record the smallest corrective action for a successor pilot, if any.
