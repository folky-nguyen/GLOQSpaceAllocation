import { create } from "zustand";

export type ViewMode = "3d" | "plan";
export type ToolMode = "select" | "space" | "level";

export type Selection =
  | { kind: "view"; id: "view-3d" | "view-plan" }
  | { kind: "level"; id: string }
  | { kind: "space"; id: string }
  | null;

type UiStore = {
  activeView: ViewMode;
  activeTool: ToolMode;
  selection: Selection;
  setActiveView: (view: ViewMode) => void;
  setActiveTool: (tool: ToolMode) => void;
  setSelection: (selection: Selection) => void;
};

export const useUiStore = create<UiStore>((set) => ({
  activeView: "plan",
  activeTool: "select",
  selection: { kind: "view", id: "view-plan" },
  setActiveView: (activeView) => set({ activeView }),
  setActiveTool: (activeTool) => set({ activeTool }),
  setSelection: (selection) => set({ selection })
}));
