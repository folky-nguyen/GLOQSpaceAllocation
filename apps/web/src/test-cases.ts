import type { ProjectDoc } from "./project-doc";
import type { ViewMode } from "./ui-store";

import levelSingleStoryRaw from "../../../supabase/sample-data/levels/single-story.json?raw";
import levelOneBasementThreeStoriesRaw from "../../../supabase/sample-data/levels/one-basement-three-stories.json?raw";
import levelThreeBasementsTwelveStoriesRaw from "../../../supabase/sample-data/levels/three-basements-twelve-stories.json?raw";
import spaceStaggerPackRaw from "../../../supabase/sample-data/spaces/apartment-stagger-pack.json?raw";
import spaceOffsetCoreRaw from "../../../supabase/sample-data/spaces/apartment-offset-core.json?raw";
import spaceDenseAngledPackRaw from "../../../supabase/sample-data/spaces/apartment-dense-angled-pack.json?raw";
import mixedCase1Raw from "../../../supabase/sample-data/mixed/case-1-single-story-tight-pack.json?raw";
import mixedCase2Raw from "../../../supabase/sample-data/mixed/case-2-one-basement-three-stories.json?raw";
import mixedCase3Raw from "../../../supabase/sample-data/mixed/case-3-three-basements-twelve-stories.json?raw";

export type SampleCaseGroup = "level" | "space" | "mixed";

export type SampleCaseManifest = {
  id: string;
  label: string;
  description: string;
  group: SampleCaseGroup;
  preferredView: ViewMode;
  preferredActiveLevelId: string;
  doc: ProjectDoc;
};

function parseSampleProjectDoc(raw: string): ProjectDoc {
  return JSON.parse(raw) as ProjectDoc;
}

function createSampleCase(
  input: Omit<SampleCaseManifest, "doc"> & { raw: string }
): SampleCaseManifest {
  return {
    id: input.id,
    label: input.label,
    description: input.description,
    group: input.group,
    preferredView: input.preferredView,
    preferredActiveLevelId: input.preferredActiveLevelId,
    doc: parseSampleProjectDoc(input.raw)
  };
}

export const LEVEL_CASES: SampleCaseManifest[] = [
  createSampleCase({
    id: "level-single-story",
    label: "Single Story",
    description: "One above-grade story with two polygon apartments for level switching baseline checks.",
    group: "level",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: levelSingleStoryRaw
  }),
  createSampleCase({
    id: "level-one-basement-three-stories",
    label: "1 Basement + 3 Stories",
    description: "Four stacked stories with repeatable polygon apartments on every level.",
    group: "level",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: levelOneBasementThreeStoriesRaw
  }),
  createSampleCase({
    id: "level-three-basements-twelve-stories",
    label: "3 Basements + 12 Stories",
    description: "Tall tower stack for active-level switching across a deeper level list.",
    group: "level",
    preferredView: "3d",
    preferredActiveLevelId: "level-1",
    raw: levelThreeBasementsTwelveStoriesRaw
  })
];

export const SPACE_CASES: SampleCaseManifest[] = [
  createSampleCase({
    id: "space-stagger-pack",
    label: "Stagger Pack",
    description: "Single-floor polygon apartments packed in a staggered cluster.",
    group: "space",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: spaceStaggerPackRaw
  }),
  createSampleCase({
    id: "space-offset-core",
    label: "Offset Core",
    description: "Offset polygon apartments around an implied shared center gap.",
    group: "space",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: spaceOffsetCoreRaw
  }),
  createSampleCase({
    id: "space-dense-angled-pack",
    label: "Dense Angled Pack",
    description: "Tighter polygon apartments with varied edge angles and denser adjacency.",
    group: "space",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: spaceDenseAngledPackRaw
  })
];

export const MIXED_CASES: SampleCaseManifest[] = [
  createSampleCase({
    id: "mixed-case-1",
    label: "Case 1: 1 Story",
    description: "Single-story tight apartment pack with a compact polygon arrangement.",
    group: "mixed",
    preferredView: "plan",
    preferredActiveLevelId: "level-1",
    raw: mixedCase1Raw
  }),
  createSampleCase({
    id: "mixed-case-2",
    label: "Case 2: 1 Basement + 3 Stories",
    description: "Four-level apartment stack with a second polygon arrangement repeated through the building.",
    group: "mixed",
    preferredView: "3d",
    preferredActiveLevelId: "level-1",
    raw: mixedCase2Raw
  }),
  createSampleCase({
    id: "mixed-case-3",
    label: "Case 3: 3 Basements + 12 Stories",
    description: "Tall apartment tower with a third polygon arrangement for high-story visibility checks.",
    group: "mixed",
    preferredView: "3d",
    preferredActiveLevelId: "level-1",
    raw: mixedCase3Raw
  })
];

export const ALL_SAMPLE_CASES: SampleCaseManifest[] = [
  ...LEVEL_CASES,
  ...SPACE_CASES,
  ...MIXED_CASES
];

export function getSampleCaseById(caseId: string): SampleCaseManifest | null {
  return ALL_SAMPLE_CASES.find((sampleCase) => sampleCase.id === caseId) ?? null;
}

export function cloneSampleProjectDoc(doc: ProjectDoc): ProjectDoc {
  return JSON.parse(JSON.stringify(doc)) as ProjectDoc;
}
