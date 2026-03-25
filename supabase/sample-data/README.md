# Sample Data

This folder stores checked-in editor sample cases for manual validation.

Rules:

- keep each case as one whole JSON document
- match the canonical `ProjectDoc` snapshot shape
- keep all numeric length values in internal decimal feet
- keep each case as one mixed scenario with levels, site polygon, setbacks, and spaces together
- store each `Space` footprint as a polygon point list
- store `sitePlan.boundary` as one simple polygon point list
- keep `sitePlan.edgeSetbacksFt.length` aligned to the boundary edge count
- do not store transient UI state here
- use stable ids once a case is introduced

Recommended layout:

- `supabase/sample-data/mixed/*.json`

This folder is not wired to live Supabase reads or writes yet.

It is still the source of truth for future UI-loaded validation cases, so the JSON shape should remain compatible with the future snapshot contract.
