import {
  repairProjectDoc,
  type ProjectDoc
} from "./project-doc";
import type { ViewMode } from "./ui-store";

import mixedCase1Raw from "../../../supabase/sample-data/mixed/case-1-single-story-angled-lot.json?raw";
import mixedCase2Raw from "../../../supabase/sample-data/mixed/case-2-one-basement-three-stories-tapered-lot.json?raw";
import mixedCase3Raw from "../../../supabase/sample-data/mixed/case-3-three-basements-twelve-stories-wide-frontage-lot.json?raw";

export type SampleCaseManifest = {
  id: string;
  label: string;
  description: string;
  preferredView: ViewMode;
  preferredActiveLevelId: string;
  doc: ProjectDoc;
};

function parseSampleProjectDoc(raw: string): ProjectDoc {
  return repairProjectDoc(JSON.parse(raw) as ProjectDoc);
}

function createSampleCase(
  input: Omit<SampleCaseManifest, "doc"> & { raw: string }
): SampleCaseManifest {
  return {
    id: input.id,
    label: input.label,
    description: input.description,
    preferredView: input.preferredView,
    preferredActiveLevelId: input.preferredActiveLevelId,
    doc: parseSampleProjectDoc(input.raw)
  };
}

export const MIXED_CASES: SampleCaseManifest[] = [
  createSampleCase({
    id: "mixed-case-1",
    label: "Case 1: Engine-Generated Site Plan",
    description: "One-level site plan generated from the layout engine on a 104 ft by 72 ft parcel with default 5 ft setbacks.",
    preferredView: "site-plan",
    preferredActiveLevelId: "level-1",
    raw: mixedCase1Raw
  }),
  createSampleCase({
    id: "mixed-case-2",
    label: "Case 2: Tapered Lot",
    description: "One basement plus three stories on a tapered five-edge site with stacked suites and editable setbacks.",
    preferredView: "site-plan",
    preferredActiveLevelId: "level-1",
    raw: mixedCase2Raw
  }),
  createSampleCase({
    id: "mixed-case-3",
    label: "Case 3: Wide Frontage",
    description: "Three basements and twelve stories on a wide-frontage lot with repeated wing layouts through the tower stack.",
    preferredView: "site-plan",
    preferredActiveLevelId: "level-1",
    raw: mixedCase3Raw
  })
];

export const ALL_SAMPLE_CASES: SampleCaseManifest[] = MIXED_CASES;

export function getSampleCaseById(caseId: string): SampleCaseManifest | null {
  return ALL_SAMPLE_CASES.find((sampleCase) => sampleCase.id === caseId) ?? null;
}

export function cloneSampleProjectDoc(doc: ProjectDoc): ProjectDoc {
  return repairProjectDoc(JSON.parse(JSON.stringify(doc)) as ProjectDoc);
}
