# 019 Engine-Generated Sample Site Plan

This task creates one representative mixed sample by running the layout engine against the current assumption-export-equivalent defaults and using the checked-in lookup data as reference context.

## Goal

- Replace one mixed fixture with a layout-engine-generated site-plan sample.
- Keep the mixed validation surface at three cases total.
- Keep the sample in `Site Plan` view with `Level 1` as the host level.

## Outcome

- `supabase/sample-data/mixed/case-1-single-story-angled-lot.json` now holds the engine-generated sample.
- `apps/web/src/test-cases.ts` labels the case as engine-generated.
- The sample uses one level, default `5 ft` setbacks, and the derived spaces from the engine export.

## Verification

- `pnpm --filter web build`
- rerun the temporary layout runner if the sample needs regeneration
