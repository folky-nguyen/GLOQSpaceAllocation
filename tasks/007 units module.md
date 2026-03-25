# 007 Units Module

## Goal

In `apps/web`, add one small units module for internal international-foot values and US feet-inch UI parsing/formatting.

The module must support parsing these inputs into internal decimal feet:

- `12'`
- `12' 6"`
- `12'-6 1/2"`
- `7 1/4"`
- `9.5'`

It must also add helpers for:

- feet `<->` meters using the international-foot constant
- rectangular area in square feet
- length formatting with sensible precision for the editor UI

Add tests for parsing, formatting, and round-trip behavior.

Also add one `Unit` button in the editor ribbon that opens a small in-app units inspector window for reading values and manually testing the implemented helpers.

This stays entirely in TypeScript. Do not move unit logic into Rust wasm or the API.

## Current Repo State

`apps/web` currently has:

- `src/project-doc.ts`
  - the canonical `ProjectDoc`, `Level`, and `Space` TypeScript types
  - numeric geometry fields already stored as `*Ft`
  - `formatFeetAndInches(lengthFt)` that rounds to the nearest whole inch
  - `getSpaceAreaSqFt(space)` as a direct width-times-depth helper
- `src/editor-shell.tsx`
  - imports `formatFeetAndInches` and `getSpaceAreaSqFt`
  - shows imperial lengths throughout the shell
  - has no units-specific inspector, modal, or test surface
- `package.json`
  - `build` and `lint` scripts only
  - no test runner for pure TypeScript logic

What is missing today:

- no dedicated `units.ts` module
- no parser for US feet-inch strings
- no meters conversion helpers
- no configurable or fraction-aware formatter
- no unit tests for length handling
- no in-app UI for manually exercising parsing/formatting/conversion behavior

## Updated Context

- TypeScript owns the canonical editor document and geometry logic for the MVP.
- The existing project model already stores lengths in decimal feet through fields like `elevationFt`, `widthFt`, and `depthFt`.
- The current formatter is too coarse for the requested inputs because it rounds to whole inches and always emits both feet and inches.
- The web app already presents imperial values in the UI, so this task is a pure domain-logic improvement inside `apps/web`.
- The repo currently has no frontend test runner, so this task needs the smallest sensible pure-logic test setup.
- The rendering crate and API do not need to know about UI parsing strings; they should continue to consume numeric feet values only.

## Scope

In scope:

- one dedicated units module in `apps/web`
- parsing US feet-inch strings into decimal feet
- formatting decimal feet into normalized US feet-inch strings
- international feet/meters conversion helpers
- rectangular area helper in square feet
- migrating current web callers to the new module
- one small ribbon-triggered units inspector window for manual testing
- pure TypeScript tests for parsing, formatting, conversions, and round-trip tolerance

Out of scope:

- changing the persisted document schema
- adding a metric UI mode or unit toggle
- locale-aware formatting outside US imperial strings
- polygon area computation
- Rust wasm or API changes
- turning the inspector into a persistent settings or project-editing dialog

## Implementation Decisions

### 1. Keep the canonical internal unit as decimal international feet

All numeric editor lengths remain plain `number` values in feet.

Use the international-foot constant only:

- `1 ft = 0.3048 m`

Do not introduce:

- US survey feet
- tagged numeric wrapper classes
- a second schema for metric storage

This keeps `ProjectDoc` unchanged and aligned with the current `*Ft` fields.

### 2. Add one dedicated `units.ts` module

Create `apps/web/src/units.ts` as the single place for length and area utilities.

Keep it flat and pure. Do not create:

- `src/utils/units/`
- class-based unit objects
- a parser framework

Suggested surface:

```ts
export const METERS_PER_FOOT = 0.3048;
export function feetToMeters(feet: number): number;
export function metersToFeet(meters: number): number;
export function getAreaSqFt(widthFt: number, depthFt: number): number;
export function parseFeetAndInches(input: string): number | null;
export function formatFeetAndInches(
  lengthFt: number,
  options?: { inchDenominator?: 2 | 4 | 8 | 16 }
): string;
```

This keeps the public API small and also reuses the existing formatter name so current call sites change minimally.

### 3. Keep `project-doc.ts` focused on document shape

`project-doc.ts` should continue to own:

- `ProjectDoc`
- `Level`
- `Space`
- `createStarterProjectDoc`
- domain-specific selectors like `getLevelSpaces`

Move generic units logic out of it.

For minimal churn:

- remove `formatFeetAndInches` from `project-doc.ts`
- keep `getSpaceAreaSqFt(space)` as a thin domain wrapper over `getAreaSqFt(space.widthFt, space.depthFt)`

This preserves a domain-friendly helper for spaces while keeping generic unit math in one place.

### 4. Support a narrow, explicit parsing grammar

The parser should accept only clear imperial forms and return `null` for invalid text instead of throwing.

Accepted shapes:

- feet only:
  - `12'`
  - `9.5'`
- feet plus inches:
  - `12' 6"`
  - `12'-6 1/2"`
- inches only:
  - `7 1/4"`
  - `6"`

Small additional allowance:

- one optional leading `-` sign for full-length values, because negative elevations are realistic for levels

Rejected by design:

- bare numbers without unit markers like `12` or `9.5`
- fractional feet forms like `12 1/2'`
- malformed fractions or mixed extra punctuation

Implementation notes:

- trim outer whitespace
- parse an optional sign once
- parse feet and inches separately
- use one small helper for `a/b` fractions
- convert the final result into decimal feet

Do not try to preserve the original input style. The parser only needs to produce normalized numeric values.

More explicit token rules:

- full input shape:
  - `[sign][feet-part][separator][inches-part]`
  - `[sign][feet-part]`
  - `[sign][inches-part]`
- `sign`:
  - optional leading `-`
- `feet-part`:
  - digits with optional decimal component followed by `'`
  - examples:
    - `12'`
    - `9.5'`
- `separator` between feet and inches:
  - optional whitespace
  - or one hyphen with optional surrounding whitespace
  - accepted examples:
    - `12' 6"`
    - `12'-6"`
    - `12' - 6 1/2"`
- `inches-part`:
  - whole inches followed by `"`
  - fraction followed by `"`
  - whole inches plus fraction followed by `"`
  - examples:
    - `6"`
    - `1/2"`
    - `6 1/2"`

Numeric rules:

- the sign applies to the whole length only
- feet and inch tokens themselves must be unsigned
- fraction denominator must be greater than `0`
- inches may be greater than or equal to `12` on input, and should still parse successfully
- parsed numeric output should normalize naturally through total-inch arithmetic

Examples of accepted normalization:

- `14"` -> `14 / 12` feet
- `5' 14"` -> `6 + 2 / 12` feet

Examples of rejection:

- `1'- -2"`
- `1' -2"`
- `1 1/2'`
- `1' 2 / 3"`
- `"`

### 5. Normalize formatting output

Formatting should convert decimal feet into normalized US feet-inch strings for display.

Default behavior:

- round to the nearest `1/16"`
- suppress zero components where possible
- carry overflow cleanly from inches into feet
- emit normalized spacing without preserving the original dash style

Expected display examples:

- `12` -> `12'`
- `12.5` -> `12' 6"`
- `12 + 6.5 / 12` -> `12' 6 1/2"`
- `7.25 / 12` -> `7 1/4"`
- `0` -> `0"`

If the value is negative, emit a single leading minus on the whole length:

- `-1.5` -> `-1' 6"`

The optional `inchDenominator` parameter should stay limited to common denominators:

- `2`
- `4`
- `8`
- `16`

Do not add fully custom arbitrary denominators for this MVP.

Formatting normalization rules:

- if the rounded result is exactly `0`, emit `0"`
- if feet is non-zero and rounded inches is `0`, emit feet only
  - example:
    - `12` -> `12'`
- if feet is `0`, emit inches only
  - example:
    - `7.25 / 12` -> `7 1/4"`
- if both feet and inches are present, emit:
  - `<feet>' <whole inches>"`
  - or `<feet>' <whole inches> <fraction>"`
- do not emit empty or redundant components
  - never `12' 0"`
  - never `0' 7 1/4"`
- if the rounded inch fraction collapses to a whole inch, carry it into the whole-inch part
- if the rounded inch total reaches `12`, carry it into feet
  - example:
    - `11' 11 15/16"` rounded to the next `1/16"` should become `12'`

Finite-number rule:

- exported numeric helpers expect finite `number` inputs
- the inspector UI should validate text inputs before calling the helpers
- this task does not add generic `NaN` or `Infinity` coercion behavior inside the units module

### 6. Keep conversion and area helpers exact and simple

`feetToMeters` and `metersToFeet` should use the exact international-foot constant.

`getAreaSqFt(widthFt, depthFt)` should simply multiply the two inputs and return square feet.

Do not add:

- square-meter helpers in this task
- area-string formatting helpers
- geometry abstractions beyond scalar helpers

### 7. Add one small ribbon-triggered units inspector

Add one small `Unit` button to the top ribbon, positioned as its own compact ribbon group beside the existing editor commands.

The button should open a lightweight floating inspector window inside the editor shell.

This inspector is for:

- reading normalized unit values
- manually testing parse behavior
- manually testing format precision
- manually testing feet/meters conversion
- manually testing square-foot area calculation

Keep it intentionally simple:

- one close button
- no routing
- no persistence
- no portal dependency
- no drag-and-drop window system

State decision:

- keep the inspector open/close state local to `editor-shell.tsx`
- keep the inspector form fields local to the inspector component
- do not add this transient diagnostic state to `ui-store.ts`

Reason:

- `ui-store.ts` currently owns core editor interaction state only
- the units inspector is a temporary shell utility, not a document-editing mode

Inspector content should cover four sections:

1. Imperial parse
   - free-text input for values like `12'-6 1/2"`
   - parse status
   - parsed decimal feet
   - converted meters
   - normalized formatted output
2. Feet format
   - decimal-feet input
   - denominator selector with `2`, `4`, `8`, `16`
   - formatted output
   - converted meters
3. Meters convert
   - meters input
   - converted feet
   - formatted imperial output
4. Area
   - width in feet
   - depth in feet
   - square-foot result

Add a small sample row in the inspector with buttons or chips for the required parser examples:

- `12'`
- `12' 6"`
- `12'-6 1/2"`
- `7 1/4"`
- `9.5'`

This makes manual smoke testing fast and deterministic.

### 8. Use Vitest as the smallest sensible test runner

Add `vitest` to `apps/web` and keep tests focused on pure logic.

Why Vitest here:

- it fits the existing Vite TypeScript stack
- it requires less setup than a browser-oriented test harness
- this task only needs Node-based pure function tests

Do not add:

- React Testing Library
- jsdom-specific setup
- snapshot tests
- a separate `vitest.config.ts` unless the default setup proves insufficient

### 9. Wire the workspace test command so the new tests are not orphaned

`apps/web/package.json` should gain a dedicated test script, and the root workspace `test` script should invoke it.

Reason:

- the repo already has a workspace-level `test` entry
- adding unit tests without wiring them into that flow would make regressions easy to miss

Keep the update narrow:

- add web test execution
- leave Rust test commands unchanged

## File Plan

### 1. Root `package.json`

Update the workspace `test` script so it includes the web unit tests in addition to the existing checks.

### 2. `apps/web/package.json`

Add:

- `vitest` as a dev dependency
- a `test` script that runs the units tests non-interactively

Do not add any other testing dependencies for this task.

### 3. `apps/web/src/units.ts`

Create the new pure helper module.

Suggested internal helpers:

- `parseFraction(text)`
- `parseInchesComponent(text)`
- `parseUnsignedDecimal(text)`
- `roundInchesToDenominator(totalInches, denominator)`
- `toMixedInchesParts(totalInchesRounded, denominator)`

Keep each helper single-purpose and local unless it is part of the public API.

Recommended implementation order inside the file:

1. declare constants and small shared types
2. implement fraction and numeric token helpers
3. implement parser helpers for feet and inches components
4. implement `parseFeetAndInches`
5. implement conversion helpers
6. implement formatter helpers and `formatFeetAndInches`
7. implement `getAreaSqFt`

### 4. `apps/web/src/project-doc.ts`

Update the file to:

- stop exporting the generic formatter
- import `getAreaSqFt` from `units.ts`
- keep `getSpaceAreaSqFt(space)` as the space-specific wrapper

No schema changes to `ProjectDoc`, `Level`, or `Space`.

### 5. `apps/web/src/editor-shell.tsx`

Update imports so display formatting comes from `units.ts`.

Add:

- a compact `Unit` ribbon button in its own group
- one local boolean state for showing the inspector
- the inspector component mount near the end of the shell markup

No broader editor-shell redesign is needed.

### 6. `apps/web/src/units-inspector.tsx`

Add one small presentational component for the floating inspector window.

Suggested props:

```ts
type UnitsInspectorProps = {
  open: boolean;
  onClose: () => void;
};
```

Implementation notes:

- return `null` when closed
- use local `useState` for the inspector fields
- derive outputs directly from `units.ts` helpers
- show a generic invalid-state message when parsing fails
- keep the layout narrow and scrollable rather than making the shell reflow

### 7. `apps/web/src/styles.css`

Add only the styles required for:

- the `Unit` ribbon group/button
- the floating inspector shell
- compact inspector field rows
- sample buttons
- read-only result blocks

Preferred layout direction:

- fixed-position or absolutely positioned panel anchored from the top-right of the workspace area
- width around `22rem` to `28rem`
- max-height with internal scroll

Do not add a full-screen modal overlay unless positioning proves unreliable.

### 8. `apps/web/src/units.test.ts`

Add colocated pure tests covering:

- parsing the required sample strings
- normalized formatting output
- conversion sanity checks
- area helper sanity check
- round-trip behavior within display precision tolerance

Do not add UI tests for the inspector in this task. The inspector exists for manual verification only.

## Inspector Behavior

The units inspector is a manual diagnostic surface, not product data entry.

Behavior requirements:

- opening it must not change the current selection, tool, or active view
- closing it must only hide the panel
- invalid text in one section must not break the other sections
- the displayed outputs should update immediately as fields change
- all displayed numeric outputs should be read-only

Default inspector seed values:

- imperial parse input:
  - `12'-6 1/2"`
- feet format input:
  - `12.5`
- meters input:
  - `3.048`
- area inputs:
  - width `24`
  - depth `18`

## Parsing Rules

For this MVP, parsing should follow these rules:

1. the presence of `'` or `"` determines whether the token is feet or inches
2. feet may be integer or decimal, but only in the feet component
3. inches may be:
   - whole inches
   - a simple fraction
   - whole inches plus a simple fraction
4. feet-plus-inches inputs may use either whitespace or a single hyphen separator
5. inches greater than `12` are accepted and normalized through total-inch arithmetic
6. total inches in a formatted result must normalize so values like `12"` roll into `1'`
7. invalid text returns `null`

This is intentionally narrower than a full CAD/Revit expression parser.

## Test Plan

Add tests only for the new pure units logic.

### Parsing tests

Verify these cases parse correctly:

- `12'` -> `12`
- `12' 6"` -> `12.5`
- `12'-6 1/2"` -> `12 + 6.5 / 12`
- `7 1/4"` -> `7.25 / 12`
- `9.5'` -> `9.5`
- `-1' 6"` -> `-1.5`

Also add a few rejection cases such as:

- `12`
- `abc`
- `1' 2/0"`
- `1' -2"`
- `1 1/2'`

### Formatting tests

Verify normalized output for:

- whole feet
- mixed feet and inches
- fractional inches
- inch-only values under one foot
- inch overflow carrying into feet
- zero
- negative values

### Conversion and area tests

Verify:

- `feetToMeters(1) === 0.3048`
- `metersToFeet(0.3048) === 1`
- `getAreaSqFt(24, 18) === 432`
- `feetToMeters(metersToFeet(x))` stays within floating-point tolerance for representative values

### Round-trip tests

Test both:

1. `parseFeetAndInches(sample)` followed by `formatFeetAndInches(...)` and parsing again
2. formatting arbitrary decimal-foot values and parsing the formatted result back

Round-trip assertions should use a tolerance based on half the display precision.

With a default precision of `1/16"`, use a maximum absolute error of:

- `1 / (12 * 32)` feet

That keeps tests aligned with intentional formatter rounding instead of demanding impossible exactness for arbitrary decimals.

## Verification Plan

When this plan is implemented, run the smallest relevant web checks:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Manual smoke:

1. run `corepack pnpm --filter web dev`
2. open the editor shell
3. verify lengths still render in the properties panel and plan cards
4. click the new `Unit` ribbon button and confirm the inspector opens without changing selection or view
5. click each sample input in the inspector and confirm parse output, meters conversion, and normalized formatting look correct
6. confirm exact-foot values no longer show unnecessary `0"` tails
7. confirm fractional values render as normalized feet-inch strings
8. change the denominator selector and confirm formatter output updates as expected
9. edit width/depth in the area section and confirm the square-foot output updates immediately
10. close the inspector and confirm the rest of the editor shell remains unchanged

## Implementation Sequence

Implement in this order:

1. add `vitest` and the web `test` script
2. build `units.ts` with parser, formatter, conversion, and area helpers
3. add `units.test.ts` and lock the pure behavior
4. migrate `project-doc.ts` and `editor-shell.tsx` to import the new formatter/helper
5. add `units-inspector.tsx` and the `Unit` ribbon button
6. add the minimal styles for the inspector
7. run web tests and web build

## Done Criteria

This task is complete when:

1. `apps/web/src/units.ts` owns the generic length and area helpers
2. internal editor values remain plain decimal international feet
3. the required sample inputs parse into correct decimal-foot values
4. the formatter emits normalized US feet-inch strings with fractional-inch precision
5. `feetToMeters`, `metersToFeet`, and `getAreaSqFt` exist and are covered by tests
6. the editor ribbon includes a working `Unit` button that opens the inspector window
7. the inspector can manually exercise parse, format, conversion, and area behaviors without mutating project data
8. current web UI imports the formatter from the new units module
9. pure tests for parsing, formatting, and round-trip behavior pass
