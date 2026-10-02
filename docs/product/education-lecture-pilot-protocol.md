Status: current
Owns: The operational protocol for the first education lecture pilot (`education-pilot-01`): authorization steps, roles, material, procedure, metrics, recording sheets, pre-registered interpretation, and evidence layout.
Does not own: The education hypotheses, boundaries, and falsifiers (`hypotheses.md`); product direction (`strategic-direction.md`); gate order (`v0.2-execution-order.md`); version semantics (`versioning.md`); Material Decisions; or any implementation.
Last reviewed against code: no new implementation is required or authorized; the pilot uses existing SRT import, session terms, review, and Project Memory plus external tools. Pre-registered thresholds owner-accepted on 2026-10-02; the baseline ASR is pending VP-ASR-001.

# Education Lecture Pilot Protocol (`education-pilot-01`)

Operational plan for the education validation shape in
[`hypotheses.md`](hypotheses.md#education-speaker-scoped-recognition-verbatim-meaning-and-three-roles).

This is a discovery pilot. It is not a Gate 8 session, does not establish
v0.2 or v0.3, and does not validate built-in ASR, recording capture, or any
product claim. Owner self-use is dogfooding evidence only.

## Questions

1. **Error type** — What share of baseline ASR errors on one teacher's lectures
   is lexical (fixable by vocabulary or correction rules) versus acoustic?
2. **Reuse** — How many repeated errors in lectures 2–4 are avoided by
   knowledge confirmed on lecture 1, through (L1) vocabulary biasing and
   (L2) correction proposals?
3. **Meaning** — How accurate are AI-inferred glosses and translations of the
   teacher's verbal habits and coined terms, as judged by the teacher?
4. **Confirmability** — How many proposed corrections can a student who is not
   the teacher confirm, with and without course materials?

## Roles

| Role | Who | Does |
|---|---|---|
| Teacher `T` | Teacher of a course the owner is enrolled in | Authorizes recording; confirms terms, verbal habits, and meanings on lecture 1; judges glosses |
| Owner student `S0` | Ezra | Records, runs the pipeline, reviews as a student (dogfood) |
| External student `S1` (≥1) | A classmate not involved in VoxProof | Reviews proposals as a student; marks unfamiliar items |
| School | Applicable office or policy | Recording and privacy policy check |

`T` must not be the owner. At least one `S1` is required for question 4.

## Authorization (before any recording)

- [ ] Check school policy on recording lectures.
- [ ] `T` agrees in writing (message or form is enough) to: recording 3–4
      lectures, local-only processing, the measurements below, and receiving
      the reviewed transcripts and term list.
- [ ] Classmates are informed at the start of the first recorded lecture;
      student speech, if any, is excluded from metrics and shared outputs.
- [ ] `S1` agrees to participate and knows their review data is aggregated.
- [ ] Record each authorization in `consent.yaml` (see Evidence Layout).

Recording happens with an ordinary device outside VoxProof. VoxProof has no
recording capture, and MD-024 recording semantics are not exercised here.

### Teacher ask (中文範本)

> 老師好，我在做一個本機運行的逐字稿校對工具，想針對這門課做一個小研究：
> 錄 3–4 堂課，錄音和逐字稿只存在我的電腦，不上傳雲端。
> 需要老師花大約 30–45 分鐘，幫忙確認第一堂課裡的專業術語、老師常用的說法和它們的意思。
> 完成後我會把整理好的逐字稿和術語表給老師，也可以給修課同學參考。
> 研究只統計數字（例如錯字率），不會公開錄音或逐字稿內容。
> 老師隨時可以要求停止或刪除。

## Material

- 3–4 consecutive lectures by `T`, same course.
- Course materials if available (syllabus, slides, handouts).
- One baseline ASR, fixed for the whole pilot and chosen by
  [VP-ASR-001](../research/VP-ASR-001-local-asr-candidate-comparison.md) on the
  lecture-1 reference windows. Record engine, model, version, language
  setting, and prompt or hotword settings in `setup.yaml`.

## Procedure

```text
0. Authorize                (checklist above)
1. Record lectures 1–4
2. Reference windows        per lecture, pick 3 windows × 5 min at fixed
                            offsets (start+10, middle, end−15 min); owner
                            produces a careful reference transcript for each
3. Baseline                 choose the ASR via VP-ASR-001 on lecture-1
                            windows, then run it on all lectures with no
                            prompt or hotwords → SRT
4. Teacher pass, lecture 1  T confirms terms, verbal habits, coined terms,
                            and their meanings → term list + meaning list
5. Knowledge layers on lectures 2–4
   L1  re-run the ASR with the confirmed term list as prompt / hotwords
   L2  import the baseline SRT into VoxProof with the confirmed terms
       (session terms / Project Memory) → correction proposals
6. Gloss test               generate AI glosses + English translations for
                            20 sampled verbal-habit / coined-term occurrences;
                            T marks each correct / partly / wrong
7. Student pass             S0 and S1 each review the lecture 2 proposals:
                            confirm / reject / cannot tell / unfamiliar;
                            first without, then with course materials
8. Summarize                fill metrics; apply pre-registered reading
```

Freeze the term list after step 4. Do not add knowledge learned from
lectures 2–4 back into L1 or L2 during the pilot.

## Metrics

| ID | Metric | Source |
|---|---|---|
| M1 | Character error rate (CER) on reference windows: baseline, L1, L2 | Steps 2, 3, 5 |
| M2 | Share of baseline window errors classified lexical vs acoustic vs other | Steps 2–3 annotation |
| M3 | Repeated errors in lectures 2–4 (same term misrecognized again) and how many L1 / L2 avoided | Step 5 |
| M4 | L2 proposal precision: confirmed ÷ surfaced | Step 7 |
| M5 | Gloss and translation accuracy: correct / partly / wrong | Step 6 |
| M6 | S1 confirmability: confirmed, cannot tell, marked unfamiliar; with vs without materials | Step 7 |
| M7 | Teacher time for step 4; owner time per lecture for steps 2–5 | Timer |

Error classification for M2:

- **lexical** — term, name, number, or coined word that a correct vocabulary
  entry would fix;
- **acoustic** — common words misheard due to accent, speed, or audio quality;
- **other** — segmentation, punctuation, overlapping speech.

## Pre-Registered Reading

Thresholds below were accepted by the owner on 2026-10-02. Do not change them after step 5 begins.

```yaml
lexical_share_supports_vocabulary_first: ">= 0.5 of M2 errors"
reuse_signal_present: "L1 or L2 avoids >= 30% of M3 repeated errors"
gloss_display_acceptable: "<= 10% of M5 judged wrong"
student_confirmability_viable: "S1 resolves >= 70% of proposals with materials"
```

- Below the lexical threshold → acoustic adaptation matters earlier than
  planned; revisit the layer order in `strategic-direction.md`.
- No reuse signal → the scoped-recognition hypothesis is weakened for this
  course; try one contrasting course before concluding.
- Gloss error above threshold → AI glosses must stay hidden or teacher-gated.
- Low confirmability → teacher confirmation or course materials are
  prerequisites for the student edition, not options.

One course with one teacher is a single case. Results guide the next step;
they do not validate the hypothesis.

## Recording Sheets

`setup.yaml`

```yaml
pilot_id: education-pilot-01
course: <code or alias>
teacher_alias: T
lectures: [L1, L2, L3, L4]
asr: {engine: , model: , version: , language: , prompt_or_hotwords: none}
voxproof_commit: <sha>
```

`per-lecture.yaml`

```yaml
lecture: L2
duration_min:
cer: {baseline: , l1: , l2: }
window_errors: {lexical: , acoustic: , other: }
repeated_errors: {total: , avoided_l1: , avoided_l2: }
l2_proposals: {surfaced: , confirmed_by_S0: , confirmed_by_S1: }
s1: {cannot_tell_without_materials: , cannot_tell_with_materials: , unfamiliar: }
owner_minutes:
notes:
```

## Evidence Layout

```text
local/education-pilot-01/        (gitignored; raw material never committed)
  consent.yaml
  audio/  srt/  reference/  terms/  glosses/
  setup.yaml
  per-lecture/L1.yaml … L4.yaml
evidence/education-pilot-01/summary.md   (aggregates only; commit after owner review)
```

`summary.md` contains only numbers, aliases, and anonymized examples approved
by `T`. Delete local raw material when `T` requests it or when the pilot ends,
whichever comes first.

## Time Budget (estimate)

| Who | Effort |
|---|---|
| Owner | ~2–3 h per lecture (ASR runs, reference windows, VoxProof review, sheets) |
| Teacher | 30–45 min once, plus ~10 min for gloss judging |
| `S1` | ~30 min |
