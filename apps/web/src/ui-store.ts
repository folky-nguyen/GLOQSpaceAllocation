import { create } from "zustand";

export type ViewMode = "3d" | "plan";
export type SelectMode = "pick-many" | "sweep";

export type Selection =
  | { kind: "view"; id: "view-3d" | "view-plan" }
  | { kind: "level"; id: string }
  | { kind: "space"; id: string }
  | { kind: "space-set"; ids: string[] }
  | null;

export function getSelectionSpaceIds(selection: Selection): string[] {
  if (!selection) {
    return [];
  }

  if (selection.kind === "space") {
    return [selection.id];
  }

  if (selection.kind === "space-set") {
    return selection.ids;
  }

  return [];
}

export function hasSelectionSpace(selection: Selection, spaceId: string): boolean {
  return getSelectionSpaceIds(selection).includes(spaceId);
}

type UiStore = {
  activeView: ViewMode;
  selectMode: SelectMode;
  selection: Selection;
  setActiveView: (view: ViewMode) => void;
  setSelectMode: (mode: SelectMode) => void;
  setSelection: (selection: Selection) => void;
};

export const useUiStore = create<UiStore>((set) => ({
  activeView: "plan",
  selectMode: "pick-many",
  selection: { kind: "view", id: "view-plan" },
  setActiveView: (activeView) => set({ activeView }),
  setSelectMode: (selectMode) => set({ selectMode }),
  setSelection: (selection) => set({ selection })
}));
