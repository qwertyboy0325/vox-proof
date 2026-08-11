Status: current
Owns: A bilingual, non-technical explanation of what VoxProof does today, why it exists, and a public articulation of the long-term direction.
Does not own: Current release acceptance, canonical product thesis or strategic-horizon status, detailed architecture, implementation status, roadmap commitments, pricing, or final market positioning.
Last reviewed against project direction: v0.1 bounded transcript review core established; v0.2 authoritative real-transcript loop under active development; long-term vision remains aspirational.

# VoxProof for Everyone / 給每個人的 VoxProof

> **VoxProof helps people use AI without surrendering control over what becomes official.**
>
> **VoxProof 讓人們能使用 AI 的能力，同時保留「什麼可以成為正式結果」的控制權。**

---

# 中文

## 一句話介紹

VoxProof 是一套以證據、來源追蹤與明確決策為核心的 AI 輔助校正系統。

它目前從逐字稿開始：AI 或規則可以指出可疑內容、提出修正建議並說明理由，但不會直接把建議當成真相。只有經過人或明確授權政策接受的決定，才能進入正式輸出。

## 它想解決什麼問題？

語音轉文字與生成式 AI 已經可以快速產生大量內容，但「產生得快」不代表「可以直接相信」。

常見問題包括：

- 人名、產品名、公司名與專業術語被辨識錯誤；
- 中英文混合內容出現奇怪拼字或同音誤判；
- AI 修改了句子，卻沒有清楚說明修改依據；
- 同一個錯誤在不同檔案中反覆出現；
- 多個工具、模型或人員提出互相衝突的版本；
- 最後留下的只有一份成品，卻不知道它是怎麼被改成這樣的；
- 使用者想讓系統越來越懂自己的領域，但不希望系統偷偷學習或自動改寫。

現有工具通常在兩個極端之間選擇：

```text
完全手動
或
讓 AI 自動決定
```

VoxProof 要建立第三條路：

```text
AI 負責發現、推理與提出候選
人或明確政策負責決定
系統負責保存證據並重建正式結果
```

## VoxProof 現在在做什麼？

目前的核心場景是「既有逐字稿的品質檢查與校正」。

一個簡化流程如下：

```text
原始逐字稿
→ 找出可能有問題的片段
→ 顯示來源位置、判斷依據與候選修正
→ 人工接受、拒絕、延後或要求手動處理
→ 產生校正後逐字稿與決策紀錄
```

關鍵原則是：

### 1. 原始來源不被偷偷改寫

原始逐字稿保留不變。校正後版本是從原始來源與已接受決定重新產生的結果。

### 2. 建議不是事實

不論建議來自規則、語音特徵、語境模型或大型語言模型，它都只是候選，不會自動取得正式權限。

### 3. 每個正式修改都有來源

系統應能回答：

- 哪一段原始內容被修改？
- 為什麼它被認為可能有問題？
- 有哪些候選方案？
- 最後選了哪一個？
- 是誰或哪個政策接受的？
- 當時使用了哪些規則、模型與資料版本？

### 4. 正式結果可以重建

只要保留原始來源、接受的決定與投影規則，就應該能重新產生相同的正式輸出，而不是依賴某次 AI 對話、摘要或暫存記憶。

## 一個生活化例子

假設一段技術影片的語音辨識結果是：

```text
我們把資料放進 post grass，再透過卡夫卡傳送。
```

VoxProof 可以提出：

- `post grass` 可能是 `Postgres` 或 `PostgreSQL`；
- `卡夫卡` 可能指 `Kafka`；
- 每個候選都連回原始字幕位置、相關術語資料與使用的判斷方式。

但 VoxProof 不會直接宣布哪一個一定正確。

它會讓使用者確認，或在使用者事先明確授權的範圍內，由已核准政策處理。最後產生的逐字稿仍能追溯每一次修改。

## 它不是什麼？

VoxProof 目前不是：

- 一套新的語音辨識引擎；
- 一鍵自動洗稿工具；
- 會自行決定內容真假的 AI；
- 會默默累積個人資料並改變行為的黑箱記憶系統；
- 會用「模型信心很高」取代人類權限的自動化平台；
- 已經完成並可大規模商用的產品。

它目前是一個正在建立中的可信校正核心與治理基礎。

## 最終目標

VoxProof 最終不只是字幕校正工具。

它的長期目標是成為：

> **概率型 AI 與正式可信紀錄之間的治理、證據與承諾層。**

完整的產品論述與策略地平線由
[VoxProof Strategic Direction](strategic-direction.md) 擁有。本頁不會把這些
方向轉換成已接受的架構或實作授權。

AI 很適合探索大量可能性：它可以發現線索、整合跨段資訊、提出多個假設、找出衝突、推測時間狀態，甚至主動修正自己的判斷。

但 AI 的推論不應因為看起來合理，就自動成為正式真相。

這個長期產品論述可用以下概念順序表達；它不是已接受的架構、資料模型或實作計畫：

```text
推論（Inference）
→ 證據（Evidence）
→ 權限（Authority）
→ 受治理的承諾（Commitment）
→ 可重建、可稽核的投影（Projection）
```

簡化成一句話：

```text
讓 AI 自由推理，但嚴格管理承諾。
```

或：

```text
loose inference
strict commitment
```

## 長期可能形成的產品能力

### 1. 能理解不同領域的長期語言記憶

使用者可以建立屬於專案、團隊、講者或領域的知識集合，例如：

- 正確人名與產品名；
- 專業術語與縮寫；
- 常見語音辨識錯誤；
- 特定講者的發音習慣；
- 組織內部用語；
- 顯示格式與文字風格偏好。

這些知識可以幫助系統提出更好的候選，但不會自動取得修改權限。

### 2. 能同時保留多個可能答案

遇到模糊內容時，系統不必過早假裝確定。

它可以保留：

- 多個競爭假設；
- 各自的證據；
- 信心與適用時間；
- 彼此衝突的原因；
- 還缺少哪些資訊。

### 3. 能處理時間、衝突與覆蓋關係

系統應能區分：

- 原始觀察；
- 後來推導的目前狀態；
- 已過期的候選；
- 被新決定取代的舊決定；
- 被撤銷的授權；
- 已失效的證據；
- 只在特定專案或時間有效的規則。

### 4. 能產生不同用途的版本

同一份不可變來源可以依照已接受決定與不同呈現政策，產生：

- 忠實逐字稿；
- 校正逐字稿；
- Clean Verbatim；
- 適合機器分析的版本；
- 適合人類閱讀的版本；
- 經授權的編輯或出版版本。

不同輸出可以長得不一樣，但都應該能追溯到同一份來源與決策歷史。

### 5. 能逐步自動化，但不越過授權

VoxProof 不反對自動化。

長期來看，重複且低風險的修改可以在明確範圍內自動處理。但自動化必須來自：

```text
可檢查的政策
+ 明確的適用範圍
+ 使用者授權
+ 完整的稽核紀錄
+ 可撤銷與可重建能力
```

而不是只因為模型分數很高。

### 6. 能在本地處理敏感內容

逐字稿、會議、研究、未發布影片與公司資料可能非常敏感。VoxProof 的長期方向是讓核心分析、資料保存與正式輸出可以在使用者控制的環境中完成。

雲端服務可以是選項，但不應是取得可信工作流程的必要條件。

## 為什麼這件事重要？

未來會有越來越多內容不是直接由人寫成，而是由語音辨識、生成式 AI、自動代理與多個資料來源共同產生。

真正困難的問題將不只是：

```text
AI 能不能產生答案？
```

而是：

```text
這個答案從哪裡來？
有哪些證據？
誰有權接受它？
它覆蓋了什麼？
它何時失效？
能不能撤銷？
能不能重新產生相同的正式結果？
```

VoxProof 想處理的就是這個缺口。

## 從字幕開始，但不止於字幕

逐字稿是一個適合開始的場景，因為它同時具有：

- 清楚的原始來源；
- 常見且可觀察的 AI 錯誤；
- 人類可以判斷的修正；
- 對專業術語與上下文的高度需求；
- 可測量的品質差異；
- 明確的輸出結果。

如果這套機制能在逐字稿中成立，它未來可以成為更廣泛的可信 AI 工作流基礎：任何需要從大量機器建議中產生正式、可稽核、可重建結果的地方，都可能使用相同原則。

## 我們最終想達成的狀態

我們希望未來的使用者可以放心地說：

> 我可以讓 AI 大膽地幫我找問題、建立假設、整合知識與提出修改，因為我知道它不能偷偷改變正式結果。每一個被接受的改變都有證據、有權限、有範圍，而且整個結果可以重新驗證與重建。

這就是 VoxProof 的最終方向。

---

# English

## In one sentence

VoxProof is an AI-assisted correction system built around evidence, source traceability, and explicit authority.

It currently starts with transcripts: models and deterministic tools may identify suspicious content, propose corrections, and explain their reasoning, but they do not get to declare their own output true. Only human-approved or explicitly authorized decisions may enter an official result.

## What problem does it solve?

Speech recognition and generative AI can produce content quickly, but fast generation is not the same as trustworthy output.

Common problems include:

- names, products, companies, and specialist terms are transcribed incorrectly;
- mixed-language content produces strange spellings or phonetic substitutions;
- an AI rewrites text without clearly explaining why;
- the same mistakes reappear across files;
- different tools, models, or people produce conflicting versions;
- only the final file survives, while the reasoning and decision history disappear;
- users want a system to learn their domain without silently learning or rewriting on their behalf.

Most tools force a choice between two extremes:

```text
fully manual work
or
letting AI decide automatically
```

VoxProof is building a third path:

```text
AI discovers, reasons, and proposes
people or explicitly authorized policies decide
the system preserves evidence and rebuilds the official result
```

## What does VoxProof do today?

The current core use case is quality assurance and correction for an existing transcript.

A simplified flow is:

```text
original transcript
→ identify potentially incorrect spans
→ show source location, evidence, and candidate corrections
→ a person accepts, rejects, defers, or handles the case manually
→ produce a reviewed transcript and decision record
```

The core principles are:

### 1. The original source is never silently overwritten

The original transcript remains unchanged. A reviewed version is derived from the source and the accepted decisions.

### 2. A suggestion is not a fact

Whether a suggestion comes from rules, phonetic analysis, contextual models, or a large language model, it remains a proposal. It does not automatically gain authority.

### 3. Every official change has provenance

The system should be able to answer:

- Which source span changed?
- Why was it considered suspicious?
- Which alternatives existed?
- Which alternative was selected?
- Who or what authorized the decision?
- Which rules, models, and data revisions were used?

### 4. Official output can be rebuilt

Given the immutable source, accepted decisions, and projection policy, the system should reproduce the same official output. It must not depend on a forgotten chat session, a model summary, or a mutable cache.

## A simple example

Suppose a technical video is transcribed as:

```text
We put the data in post grass and send it through calf car.
```

VoxProof may propose that:

- `post grass` refers to `Postgres` or `PostgreSQL`;
- `calf car` refers to `Kafka`;
- each proposal is linked to the original subtitle span, relevant terminology, and the method that produced it.

VoxProof does not automatically declare one answer correct.

A person confirms the decision, or an already approved policy handles it within an explicitly authorized scope. The resulting transcript retains a traceable history for every change.

## What is it not?

VoxProof is not currently:

- a new speech-recognition engine;
- a one-click automatic rewriting tool;
- an AI that decides what is true;
- a black-box memory system that silently changes behavior;
- an automation platform where a high model score replaces human authority;
- a finished, production-scale commercial product.

Today it is a developing trusted-correction core and governance foundation.

## The ultimate goal

VoxProof is not ultimately just a subtitle correction tool.

Its long-term goal is to become:

> **The governance, evidence, and commitment layer between probabilistic AI and authoritative records.**

The full product thesis and strategic-horizon boundary are owned by
[VoxProof Strategic Direction](strategic-direction.md). This page does not
turn them into accepted architecture or implementation authority.

AI is well suited to exploring possibilities. It can discover signals, connect distant evidence, propose competing hypotheses, detect conflicts, reason about time, and revise its own conclusions.

But a plausible inference should not automatically become official truth.

This long-term product thesis can be expressed with the following conceptual
path. It is not accepted architecture, a data model, or an implementation plan:

```text
Inference
→ Evidence
→ Authority
→ Commitment
→ rebuildable, auditable Projection
```

In one phrase:

```text
let AI reason freely, but govern commitment strictly
```

Or:

```text
loose inference
strict commitment
```

## Long-term product capabilities

### 1. Persistent, scoped language knowledge

Users may build knowledge collections for a project, team, speaker, or domain, including:

- correct names and product identities;
- specialist terms and abbreviations;
- recurring speech-recognition errors;
- speaker-specific pronunciation patterns;
- organization-specific vocabulary;
- presentation and formatting preferences.

This knowledge can improve proposals without automatically gaining edit authority.

### 2. Multiple competing answers can remain visible

When evidence is ambiguous, the system should not pretend to know too early.

It may retain:

- competing hypotheses;
- supporting evidence for each;
- confidence and temporal scope;
- explicit conflicts;
- missing information required for resolution.

### 3. Time, conflict, and supersession can be represented explicitly

The system should distinguish:

- immutable observations;
- inferred current state;
- expired beliefs;
- decisions superseded by later decisions;
- revoked authorization;
- invalidated evidence;
- policies valid only for a particular project, session, or period.

### 4. One source can produce multiple governed outputs

The same immutable source may produce:

- a verbatim transcript;
- a reviewed transcript;
- clean verbatim;
- a machine-oriented representation;
- a human presentation view;
- an authorized editorial or publication version.

The outputs may look different, but each should remain traceable to the same source and decision history.

### 5. Automation can grow without bypassing authority

VoxProof is not opposed to automation.

Repeated, low-risk transformations may eventually be handled automatically within explicit boundaries. But automation must come from:

```text
inspectable policy
+ explicit scope
+ user authorization
+ complete audit history
+ revocation and rebuild capability
```

—not merely from a high model score.

### 6. Sensitive work can remain local

Transcripts, meetings, research, unpublished media, and company information may be highly sensitive. VoxProof's long-term direction is to keep core analysis, evidence, and official materialization available in an environment controlled by the user.

Cloud services may be optional, but they should not be required for a trustworthy workflow.

## Why does this matter?

More and more content will be created through speech recognition, generative AI, autonomous agents, and combined data sources.

The difficult question will no longer be only:

```text
Can AI generate an answer?
```

It will be:

```text
Where did this answer come from?
What evidence supports it?
Who had authority to accept it?
What did it supersede?
When does it expire?
Can it be revoked?
Can the same official result be verified and rebuilt?
```

VoxProof is intended to fill that gap.

## Starting with transcripts, but not ending there

Transcripts are a useful starting point because they provide:

- a clear original source;
- common and observable machine errors;
- corrections that people can judge;
- strong dependence on terminology and context;
- measurable quality differences;
- a concrete final output.

If the mechanism works for transcripts, the same principles may support a much broader class of trusted AI workflows: any system that must turn large volumes of machine proposals into official, auditable, and reproducible results.

## The future state we want

We want users to be able to say:

> I can let AI aggressively search for problems, form hypotheses, combine knowledge, and propose changes because I know it cannot silently alter the official result. Every accepted change has evidence, authority, scope, and a result that can be independently verified and rebuilt.

That is the ultimate direction of VoxProof.
