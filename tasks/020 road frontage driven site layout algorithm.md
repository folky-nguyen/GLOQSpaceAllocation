# 020 Road Frontage Driven Site Layout Algorithm

This task note defines one lean algorithm for deriving:

- `building footprint`
- `surface parking stalls`
- `external walkways`

from a parcel polygon whose edges carry one boolean road-frontage flag.

The note is design-only in this pass. It does not add runtime code yet.

## Goal

Describe one deterministic TypeScript-owned algorithm that uses:

- `parcel boundary`
- `per-edge setback`
- `per-edge road-frontage true/false`
- a small set of sizing inputs

to produce one site layout proposal without creating a second authored BIM schema.

## Why This Note Exists

The current repo already derives a `building footprint` from `sitePlan.boundary` and `edgeSetbacksFt`.

What is still missing for a frontage-aware site-layout pass:

- one boolean marker per parcel edge for road adjacency
- one repeatable way to decide which edge is the public front
- one repeatable way to place the building so it addresses the street
- one repeatable way to place surface parking so it does not dominate the public frontage
- one repeatable way to connect parking and frontage to the building with outside walkways

## Scope

In scope:

- one future-facing data addition to `SitePlan`
- frontage classification from edge booleans
- deterministic footprint archetype selection
- deterministic surface-parking packing
- deterministic outside-walkway routing
- failure and fallback rules

Out of scope:

- freeform site-design editing tools
- multi-building compounds
- structured parking
- fire-truck turning analysis
- ADA-code-complete parking and ramp design
- retaining walls, grading, or stormwater design
- code implementation in this task

## Proposed Canonical Input

Keep the current TypeScript-owned `SitePlan` seam and extend it with one aligned boolean array:

```ts
type SitePlan = {
  levelId: string;
  boundary: Point2Ft[];
  edgeSetbacksFt: number[];
  edgeRoadFrontageFlags: boolean[];
};
```

Rules:

- `boundary[index]` to `boundary[index + 1]` owns `edgeSetbacksFt[index]`
- that same edge also owns `edgeRoadFrontageFlags[index]`
- `edgeRoadFrontageFlags.length` must match the edge count
- repair missing or invalid flag values to `false`
- if every flag is `false`, promote the longest edge to `true` and emit one diagnostic

## Extra Sizing Inputs Required

Road-frontage flags are not enough to produce a useful site layout by themselves.

The algorithm also needs a small set of target values:

```ts
type FrontageLayoutInputs = {
  targetBuildingAreaSqFt: number;
  targetParkingStallCount: number;
  preferredBuildingDepthFt: number;
  frontageWalkwayWidthFt: number;
  sideWalkwayWidthFt: number;
  stallWidthFt: number;
  stallDepthFt: number;
  driveAisleWidthFt: number;
};
```

Lean defaults:

- `preferredBuildingDepthFt = 60`
- `frontageWalkwayWidthFt = 8`
- `sideWalkwayWidthFt = 6`
- `stallWidthFt = 9`
- `stallDepthFt = 18`
- `driveAisleWidthFt = 24`

## Derived Output Shape

This note assumes the layout result stays derived until a later task proves what must become authored:

```ts
type DerivedSiteLayout = {
  footprint: Point2Ft[] | null;
  parkingStalls: Point2Ft[][];
  driveAisles: Point2Ft[][];
  walkways: Point2Ft[][];
  entryEdgeIndices: number[];
  diagnostics: string[];
};
```

## Main Design Rules

### 1. Frontage edges define public address

An edge with `edgeRoadFrontageFlags[index] === true` is treated as a public street edge.

Public-facing rules:

- at least one building entry should address a frontage edge
- a continuous frontage walkway should be kept between building and road
- surface parking should avoid occupying the primary frontage band unless no side or rear solution fits

### 2. The building should hold the street, not float at the center by default

If at least one road frontage exists, the first candidate footprint should be anchored near the primary frontage instead of being centered inside the parcel.

### 3. Parking should move to side or rear residual zones first

Parking priority:

1. rear zone
2. side zone
3. secondary frontage zone
4. primary frontage zone only as the last fallback

### 4. Walkways must connect both road and parking to the building

The layout is incomplete unless the algorithm can route:

- one public walk from the road frontage to a building entry
- one parking walk from the stall field to a building entry

## Step 1. Normalize The Parcel

Start from the current site-plan rules:

1. normalize duplicate points away
2. require at least `3` valid points
3. repair `edgeSetbacksFt`
4. repair `edgeRoadFrontageFlags`
5. normalize polygon winding to one stable direction

Then derive edge records:

```ts
type ParcelEdge = {
  index: number;
  start: Point2Ft;
  end: Point2Ft;
  midpoint: Point2Ft;
  lengthFt: number;
  setbackFt: number;
  hasRoadFrontage: boolean;
  tangent: Point2Ft;
  inwardNormal: Point2Ft;
};
```

## Step 2. Build Frontage Chains

Contiguous `true` edges should be grouped into one `frontage chain`.

Example:

- `[true, true, false, false]` -> one frontage chain with two edges
- `[true, false, true, false]` -> two separate frontage chains

For each chain compute:

- total chain length
- centroid of the chain midpoints
- average tangent

Then classify:

- `primary frontage chain`: longest frontage chain
- `secondary frontage chains`: all other frontage chains

If there is no explicit `true` edge after repair:

- promote the longest parcel edge to `primary frontage chain`
- add diagnostic: `No frontage flag was true; longest parcel edge was used as frontage.`

## Step 3. Classify The Remaining Edges

Non-frontage edges become `side` or `rear` by their relation to the primary frontage.

Lean rule:

- compute the average inward normal of the primary frontage chain
- the non-frontage chain whose outward normal is most opposite that direction becomes `rear`
- remaining non-frontage chains become `side`

This classification is enough for a first deterministic pass and avoids inventing zoning parcels or street-centerline geometry.

## Step 4. Build The Base Buildable Envelope

Use the current inset-footprint logic to derive:

`baseEnvelope = inset(parcelBoundary, edgeSetbacksFt)`

If the inset fails:

- return `footprint: null`
- return empty parking and walkway arrays
- keep the diagnostic from the inset step

The rest of the algorithm only runs on a valid `baseEnvelope`.

## Step 5. Reserve Frontage And Access Bands

Before placing the building, reserve two kinds of strips.

### Frontage walkway band

For every frontage edge, create an inward strip:

`frontageWalkwayWidthFt`

Rules:

- this band is kept clear for pedestrian movement and entry forecourt
- parking stalls may not occupy this band
- drive aisles may cross it only at explicit curb-cut access points

### Service access band

At one selected vehicular-access edge, reserve one strip for driveway connection from the road to the parking field.

Vehicular-access priority:

1. shortest secondary frontage edge
2. shortest edge in the primary frontage chain
3. shortest side edge if local policy allows side access

## Step 6. Choose A Footprint Archetype From The Frontage Pattern

To keep the logic lean, do not solve arbitrary massing first.

Pick one footprint archetype from the frontage-chain pattern:

### Case A. One frontage chain -> `street bar`

Use when there is one contiguous public face.

Intent:

- building runs roughly parallel to the road
- parking moves behind the building

### Case B. Two adjacent frontage chains -> `corner L`

Use when the parcel has a corner-lot condition.

Intent:

- the building addresses both streets
- parking tucks behind the inside elbow or rear

### Case C. Two opposite frontage chains -> `through bar`

Use when the parcel fronts two opposite roads.

Intent:

- building spans between the two public sides
- parking shifts to one or both side pockets

### Case D. Three or more frontage chains -> `compact block`

Use when the parcel is highly exposed to road edges.

Intent:

- preserve public edge continuity
- keep parking in the largest internal or side residual pocket

## Step 7. Generate Candidate Building Footprints

For each archetype generate one or more candidates inside the `baseEnvelope`.

### Candidate rule for `street bar`

1. offset the primary frontage chain inward by `frontageWalkwayWidthFt`
2. project a depth cap inward by `preferredBuildingDepthFt`
3. clip the strip against the `baseEnvelope`
4. if the clipped area is larger than `targetBuildingAreaSqFt`, trim from the least-public end
5. if the clipped area is too small, allow depth growth until:
   - target area is reached, or
   - the rear parking reserve would be destroyed

### Candidate rule for `corner L`

1. generate one bar from each adjacent frontage chain
2. union the two bars
3. clip the union to the `baseEnvelope`
4. trim inner overlap if it blocks the preferred parking court

### Candidate rule for `through bar`

1. create two frontage strips from the opposite frontage chains
2. solve one connecting bar that preserves public entries on both sides
3. keep at least one side residual pocket large enough for parking modules

### Candidate rule for `compact block`

1. shrink the `baseEnvelope` by the frontage walkway band on all frontage edges
2. inscribe the largest simple polygon or rectangle aligned to the primary frontage tangent
3. trim to `targetBuildingAreaSqFt`

## Step 8. Score Building Candidates

Choose the best footprint by score, not by first fit.

Suggested score:

```txt
score =
  + 6 * frontageContactLengthFt
  + 4 * achievedBuildingAreaRatio
  + 3 * protectedPrimaryFrontageRatio
  + 2 * rearParkingCapacityRatio
  - 5 * parkingOnPrimaryFrontageRatio
  - 4 * walkwayConflictCount
  - 8 * invalidGeometryPenalty
```

Preferred outcome:

- the footprint touches or nearly parallels the primary frontage
- the primary frontage keeps a clear public walk
- one parking pocket remains viable

## Step 9. Derive Surface Parking Zones

After the footprint is chosen, derive residual parking polygons:

`parkingResidual = parcelBoundary - setbacks - buildingFootprint - frontageWalkwayBand - requiredWalkways`

Classify each residual polygon as:

- `rear pocket`
- `side pocket`
- `secondary frontage pocket`
- `primary frontage pocket`

Then rank them using the same priority:

1. rear
2. side
3. secondary frontage
4. primary frontage

## Step 10. Pack Parking Modules

For each ranked parking pocket, try two orientations:

- stalls perpendicular to the primary frontage tangent
- stalls parallel to the primary frontage tangent

Use standard module templates:

- single-loaded module depth = `stallDepthFt + driveAisleWidthFt`
- double-loaded module depth = `stallDepthFt * 2 + driveAisleWidthFt`

Packing loop:

1. fit the largest inscribed rectangle aligned to the test orientation
2. compute how many whole parking modules fit
3. emit stall rectangles row by row
4. emit one drive aisle polygon per module
5. stop when `targetParkingStallCount` is reached

Scoring rule for each parking orientation:

- more stalls is better
- fewer drive-aisle turns is better
- keeping stalls off the primary frontage is strongly better

## Step 11. Route Outside Walkways

Walkways are routed after the building and parking fields are known.

Required paths:

- `frontage entry walk`: primary frontage edge to main building entry
- `parking entry walk`: nearest parking aisle node to the same building entry

Lean routing rule:

1. place the main building entry at the midpoint of the longest footprint edge facing the primary frontage
2. project one straight walk to the frontage walkway band
3. connect the parking field by the shortest orthogonal polyline that:
   - stays outside stall rectangles
   - crosses drive aisles only at marked crossing segments
   - reaches the same entry node

Perimeter rule:

- keep at least `sideWalkwayWidthFt` clear along the building edges used by entries or parking access

## Step 12. Fallback Rules

If the preferred layout cannot fit both building and parking, fail gracefully in this order:

### Fallback 1. Keep the building on the frontage, reduce parking count

If the parking target does not fit while the building still does:

- keep the frontage-respecting building
- reduce parking to the maximum fit count
- emit diagnostic

### Fallback 2. Keep parking off the primary frontage, deepen the building less

If the building target area and parking target conflict:

- reduce achieved building area before moving parking onto the primary frontage

### Fallback 3. Allow limited parking on secondary frontage

Only after rear and side pockets fail.

### Fallback 4. Allow parking on primary frontage

Only when:

- no rear pocket fits any module
- no side pocket fits any module
- no secondary frontage pocket fits any module

Emit a high-severity diagnostic because this is usually a poor urban result.

## Step 13. Validation Rules

The derived result should be rejected when any of these are true:

- footprint polygon is invalid
- footprint extends outside the setback envelope
- parking stalls overlap the footprint
- parking stalls overlap the frontage walkway band
- walkway segments cross the footprint
- driveway connection to a road frontage cannot be formed

## Deterministic Pseudocode

```ts
function deriveFrontageDrivenSiteLayout(sitePlan, inputs): DerivedSiteLayout {
  const parcel = normalizeParcel(sitePlan);
  const frontageChains = buildFrontageChains(parcel.edges);
  const edgeRoles = classifyEdgeRoles(parcel.edges, frontageChains.primary);
  const baseEnvelope = insetParcelBySetbacks(parcel);

  if (!baseEnvelope.ok) {
    return failedLayout(baseEnvelope.error);
  }

  const frontageBands = buildFrontageWalkBands(frontageChains, inputs.frontageWalkwayWidthFt);
  const accessEdge = chooseVehicularAccessEdge(parcel.edges, frontageChains, edgeRoles);
  const buildingCandidates = generateFootprintCandidates(
    baseEnvelope.polygon,
    frontageChains,
    edgeRoles,
    frontageBands,
    inputs
  );

  const chosenFootprint = chooseBestFootprint(buildingCandidates, inputs);
  if (!chosenFootprint) {
    return failedLayout("No valid frontage-respecting building footprint could be derived.");
  }

  const parkingPockets = deriveParkingResiduals(
    parcel.boundary,
    baseEnvelope.polygon,
    chosenFootprint,
    frontageBands,
    accessEdge
  );

  const parkingLayout = packParking(parkingPockets, frontageChains.primary, inputs);
  const walkways = routeWalkways(chosenFootprint, parkingLayout, frontageBands, frontageChains.primary, inputs);

  return validateAndFinalizeLayout(chosenFootprint, parkingLayout, walkways);
}
```

## Suggested Repo Boundary

When this note becomes code, keep ownership lean:

- `apps/web/src/project-doc.ts`
  - canonical `SitePlan` repair
  - frontage-flag repair
  - buildable envelope helpers
  - frontage-chain helpers
  - derived footprint, parking, and walkway helpers
- `apps/web/src/editor-shell.tsx`
  - render overlays
  - inspector inputs
  - selection and diagnostics display

Do not move this logic into:

- Rust wasm
- Rust API
- Supabase SQL

## Verification Plan For Future Implementation

Pure logic tests should cover:

- frontage-flag repair when the array length is short
- all-false fallback to the longest frontage edge
- one-frontage `street bar` selection
- corner-lot `corner L` selection
- parking staying off the primary frontage when rear parking fits
- walkway routing from frontage and parking to the same entry
- graceful reduction of parking count when the target does not fit

## Implementation Status

This task is documentation-only.

No runtime code has been added yet.
