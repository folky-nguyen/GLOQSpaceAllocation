# KL001 KL Glossary Structure And Update Workflow

This task exists to tighten how the repo defines and maintains specialized technical terms.

Today, `KL.md` is a broad glossary. The requested change is to make it more deliberate:

- `KL.md` should define selected technical concepts tied to repo code, calculation logic, or specialized types/functions/variables
- `KL.md` should not try to enumerate every common variable or obvious programming term
- each concept should separate a repo-grounded definition from a human/domain definition
- `AGENTS.md` should explicitly require a `KL.md` update whenever a task introduces a new concept that should remain discoverable

## Goal

Reshape the glossary workflow so `KL.md` becomes the maintained home for specialized repo concepts, with a consistent `AI` and `HM` structure per concept, and with an explicit upkeep rule in `AGENTS.md`.

## Requested KL Meaning

For this task, treat `KL.md` as:

- the place to define selected technical concepts behind specialized variables, functions, and types
- especially concepts that matter to calculations, derived geometry, runtime interpretation, or repeated repo understanding
- not a place to list all common variables or obvious implementation details

More concretely, `KL.md` should help a future contributor answer:

- what this concept means in the current repo
- why the term matters to implementation or reasoning
- where the repo currently expresses that meaning
- whether the repo meaning differs from a broader professional or domain meaning

## Entry Structure

The glossary should move to a numbered section structure.

Each section contains one or more concepts.

Each concept should use this shape:

```md
## 1. <Section Name>

### `<Concept Name>`

- AI: definition based on how the current repo code behaves. Codex can maintain and update this.
- HM: definition based on real-world technical or domain knowledge. Human-authored. Use `null` when not provided yet.
```

Example:

```md
## 1. Geometry Terms

### `ProjectDoc`

- AI: The canonical TypeScript document that the current editor code reads, edits, and persists as snapshot JSON.
- HM: null
```

Top-level numbering should be used for stable groups such as:

- workflow and repo terms
- document and domain terms
- geometry and units terms
- runtime and rendering terms
- persistence terms

The exact section names can stay lean, but the grouping should be stable enough that future additions do not reshuffle the whole file every task.

## Clarifications Needed By The Plan

### 1. What counts as a glossary-worthy concept

Add a term to `KL.md` only when at least one of these is true:

- the term carries specialized repo meaning that is not obvious from the name alone
- the term is important to calculations, geometry derivation, persistence shape, runtime phase, or ownership boundaries
- the term appears across multiple files, tasks, or discussions and is likely to be reused
- misunderstanding the term would likely cause incorrect implementation or review decisions
- the term is a repo-specific workflow concept that future tasks will keep referencing

Do not add a term just because it exists in code.

### 2. What should stay out of `KL.md`

Do not add:

- common language or framework words like `state`, `props`, `component`, `function`, or `router`
- one-off local variables
- obvious names whose meaning is fully clear from nearby code
- every exported type or helper by default
- purely temporary implementation details with no repeated reasoning value

### 3. How to write `AI`

`AI` should:

- describe the concept from actual current repo behavior, not from intended future architecture
- prefer plain language grounded in ownership, inputs, outputs, and effect on repo behavior
- mention the current implementation seam or file only when it helps disambiguate the meaning
- be safe for Codex to maintain in later tasks without needing human domain judgment

`AI` should not:

- invent broader real-world meaning
- drift into speculative design intent
- become a file inventory or code dump

### 4. How to write `HM`

`HM` should:

- capture the broader human-authored technical or professional definition when that adds value
- stay short and concept-focused rather than explaining repo implementation
- use `null` when no human/domain definition has been provided yet

`HM` exists to preserve real-world meaning without forcing Codex to guess it.

### 5. If `AI` and `HM` differ

The plan should state this explicitly:

- `AI` is authoritative for how the current repo behaves
- `HM` is contextual and may be broader or stricter than the repo
- if the repo is knowingly simplified, keep both entries instead of forcing them to match

Example pattern:

- `AI`: describes the simplified MVP behavior in this repo
- `HM`: describes the fuller domain meaning, or `null` if not yet written

### 6. How concept names should be titled

Use:

- backticked identifiers for code-native names such as `ProjectDoc`, `activeView`, or `snapshot`
- plain text only when the concept is broader than one identifier and the repo already refers to it that way

Prefer one concept per heading.

If two identifiers are only useful together, either:

- define them separately
- or define the broader concept once and list the paired identifiers inside the `AI` text

### 7. How existing `KL.md` content should be migrated

This task should not require preserving the current structure verbatim.

Instead, the implementation should:

- keep the high-signal concepts already present
- rewrite them into the new `AI` / `HM` shape
- merge duplicates or near-duplicates if they do not need separate entries
- drop low-value terms that fail the new inclusion rule

The migration target is a cleaner glossary, not a one-to-one format conversion.

## Scope

In scope:

- update `KL.md` purpose so it is selective rather than exhaustive
- define the `AI` and `HM` meaning clearly
- reshape glossary entries to the new numbered-section format
- update `AGENTS.md` so new concept creation triggers a same-task `KL.md` update
- keep the change documentation-only

Out of scope:

- adding every existing variable, function, or type in the repo
- rewriting product architecture
- changing runtime behavior

## Implementation Decisions

### 1. `KL.md` is selective

The glossary should only keep terms that improve repeated repo understanding.

Do not use it as a dump for ordinary variable names or generic programming vocabulary.

### 2. `AI` is repo-grounded

`AI` should describe the concept from the current implementation as it exists in this repo.

This gives Codex a safe place to maintain definitions based on real code behavior instead of generic textbook language.

### 3. `HM` is human/domain-grounded

`HM` should describe the real-world technical meaning when a human wants to provide a stronger professional or domain definition.

If no human definition exists yet, store `null`.

### 4. `AGENTS.md` must require glossary upkeep

If a task adds a new recurring repo concept, or clarifies a specialized term that future tasks will likely need, the same task should update `KL.md`.

This should be added to the documented workflow rather than relying on memory.

### 5. The upkeep trigger must be explicit, not fuzzy

The `AGENTS.md` update should make clear that `KL.md` must be updated in the same task when any of the following happen:

- a new glossary-worthy concept is introduced
- an existing concept changes meaning in code
- a task establishes a new repo term that future notes or reviews will likely reuse
- a rename changes the canonical term contributors should use

No `KL.md` update is required for:

- ordinary refactors with no terminology change
- trivial variable additions
- generic framework code that adds no repo-specific concept

### 6. `KL.md` should support both repo and workflow concepts

The note should keep room for:

- product and domain concepts like `ProjectDoc`, `Level`, `snapshot`
- runtime concepts like `activeView`, `derived`, `transient`
- workflow concepts like `task note` or future recurring repo process terms

This avoids splitting glossary ownership across multiple docs for the same style of recurring term.

## File Plan

### 1. `KL.md`

Expected updates:

- rewrite the file purpose
- add the selective glossary rule
- add the `AI` / `HM` definition rule
- add a short inclusion/exclusion rule so contributors know what belongs in the file
- convert existing terms to the new entry format where useful
- prune or merge terms that do not meet the new bar
- keep section ordering stable enough for future tasks to append without rethinking the whole document

Suggested implementation shape:

```md
# KL

Purpose statement...

How to add a term...

## 1. Workflow Terms
### `task note`
- AI: ...
- HM: null

## 2. Domain Terms
### `ProjectDoc`
- AI: ...
- HM: null
```

### 2. `AGENTS.md`

Expected updates:

- add a workflow or rules step requiring `KL.md` updates when new concepts are introduced
- keep the rule scoped to specialized repo concepts, not every normal variable name
- clarify that the update happens in the same task that introduces or changes the concept
- keep the language practical so future task notes can reference it directly

### 3. `MP.md`

Expected updates if needed:

- update the document index or file-role wording if the KL glossary workflow becomes easier to discover after this task

## Verification Plan

When this task is implemented, verify that:

1. `KL.md` explicitly says it is selective and not exhaustive
2. `KL.md` explains `AI` and `HM` and uses `null` for missing human definitions
3. `AGENTS.md` explicitly says new glossary-worthy concepts must update `KL.md`
4. `KL.md` makes the include/exclude rule concrete enough that contributors can decide whether a term belongs there
5. the resulting glossary format is easy to extend without listing every common variable
6. if `AI` and `HM` differ, the document makes their roles non-conflicting

## Done Criteria

This task is complete when:

1. `KL.md` is reframed as a selective glossary for specialized repo concepts
2. each kept concept follows the `AI` / `HM` structure
3. `AGENTS.md` includes the glossary-maintenance rule for new concepts
4. the task note makes inclusion, exclusion, and update-trigger rules explicit enough to implement without guesswork
5. `MP.md` is reviewed and updated if this task changes glossary discovery paths

## Implementation Status

Implemented in:

- `KL.md`
- `AGENTS.md`
- `MP.md`

Result:

- `KL.md` now uses a selective numbered glossary structure with concrete include/exclude guidance and `AI` / `HM` entries.
- stale low-signal glossary content such as file-inventory-style seams was removed from `KL.md`.
- `AGENTS.md` now requires same-task `KL.md` updates when glossary-worthy concepts are introduced, renamed, or change meaning.
- `MP.md` now describes `KL.md` as the selective glossary for specialized repo concepts so future discovery stays explicit.
