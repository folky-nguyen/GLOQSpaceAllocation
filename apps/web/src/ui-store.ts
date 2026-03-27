import { create } from "zustand";

export type ViewId = "view-3d" | "view-plan" | "view-site-plan";
export type ViewMode = "3d" | "plan" | "site-plan";
export type SelectionElementRef = { kind: "space"; id: string };

export type Selection =
  | { kind: "view"; id: ViewId }
  | { kind: "site-edge"; edgeIndex: number }
  | { kind: "level"; id: string }
  | { kind: "element"; element: SelectionElementRef }
  | { kind: "element-set"; elements: SelectionElementRef[] }
  | null;

export function getSelectionElementKey(element: SelectionElementRef): string {
  return `${element.kind}:${element.id}`;
}

export function getSelectionElements(selection: Selection): SelectionElementRef[] {
  if (!selection) {
    return [];
  }

  if (selection.kind === "element") {
    return [selection.element];
  }

  if (selection.kind === "element-set") {
    return selection.elements;
  }

  return [];
}

export function createSelectionFromElements(elements: SelectionElementRef[], fallback: Selection): Selection {
  const uniqueElements: SelectionElementRef[] = [];
  const seen = new Set<string>();

  for (const element of elements) {
    const key = getSelectionElementKey(element);

    if (seen.has(key)) {
      continue;
    }

    seen.add(key);
    uniqueElements.push(element);
  }

  if (uniqueElements.length === 0) {
    return fallback;
  }

  if (uniqueElements.length === 1) {
    return { kind: "element", element: uniqueElements[0] };
  }

  return { kind: "element-set", elements: uniqueElements };
}

export function getSelectionSpaceIds(selection: Selection): string[] {
  return getSelectionElements(selection).flatMap((element) => (
    element.kind === "space" ? [element.id] : []
  ));
}

export function hasSelectionElement(selection: Selection, element: SelectionElementRef): boolean {
  return getSelectionElements(selection).some((candidate) => (
    candidate.kind === element.kind && candidate.id === element.id
  ));
}

export function hasSelectionSpace(selection: Selection, spaceId: string): boolean {
  return hasSelectionElement(selection, { kind: "space", id: spaceId });
}

type UiStore = {
  activeView: ViewMode;
  selection: Selection;
  setActiveView: (view: ViewMode) => void;
  setSelection: (selection: Selection) => void;
  resetSessionUi: () => void;
};

const defaultSessionUiState: Pick<UiStore, "activeView" | "selection"> = {
  activeView: "plan",
  selection: { kind: "view", id: "view-plan" }
};

export const useUiStore = create<UiStore>((set) => ({
  ...defaultSessionUiState,
  setActiveView: (activeView) => set({ activeView }),
  setSelection: (selection) => set({ selection }),
  resetSessionUi: () => set(defaultSessionUiState)
}));
