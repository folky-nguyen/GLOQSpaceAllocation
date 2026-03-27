import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

export type PanelPosition = {
  x: number;
  y: number;
};

type PanelDragState = {
  pointerId: number;
  offsetX: number;
  offsetY: number;
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function clampPanelPosition(
  position: PanelPosition,
  panel: HTMLElement | null,
  workspace: HTMLElement | null
): PanelPosition {
  if (!panel || !workspace) {
    return position;
  }

  const maxX = Math.max(0, workspace.clientWidth - panel.offsetWidth);
  const maxY = Math.max(0, workspace.clientHeight - panel.offsetHeight);

  return {
    x: clamp(position.x, 0, maxX),
    y: clamp(position.y, 0, maxY)
  };
}

export function useDraggablePanel<T extends HTMLElement>(
  workspaceRef: RefObject<HTMLElement | null>,
  initialPosition: PanelPosition | null = null
) {
  const panelRef = useRef<T | null>(null);
  const dragStateRef = useRef<PanelDragState | null>(null);
  const [position, setPosition] = useState<PanelPosition | null>(initialPosition);

  useEffect(() => {
    if (!initialPosition) {
      return;
    }

    setPosition((current) => current ?? initialPosition);
  }, [initialPosition]);

  useEffect(() => {
    if (!initialPosition) {
      return;
    }

    const panel = panelRef.current;
    const workspace = workspaceRef.current;

    if (!panel || !workspace) {
      return;
    }

    setPosition((current) => clampPanelPosition(current ?? initialPosition, panel, workspace));
  }, [initialPosition, workspaceRef]);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const dragState = dragStateRef.current;
      const workspace = workspaceRef.current;
      const panel = panelRef.current;

      if (!dragState || !workspace || !panel) {
        return;
      }

      const workspaceRect = workspace.getBoundingClientRect();
      const nextPosition = clampPanelPosition(
        {
          x: event.clientX - workspaceRect.left - dragState.offsetX,
          y: event.clientY - workspaceRect.top - dragState.offsetY
        },
        panel,
        workspace
      );

      setPosition(nextPosition);
    };

    const handlePointerUp = (event: PointerEvent) => {
      if (dragStateRef.current?.pointerId === event.pointerId) {
        dragStateRef.current = null;
      }
    };

    const handleWindowResize = () => {
      setPosition((current) => {
        if (!current) {
          return current;
        }

        return clampPanelPosition(current, panelRef.current, workspaceRef.current);
      });
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("resize", handleWindowResize);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("resize", handleWindowResize);
    };
  }, [workspaceRef]);

  const handleHeaderPointerDown = (event: ReactPointerEvent<HTMLElement>) => {
    const panel = panelRef.current;
    const workspace = workspaceRef.current;

    if (!panel || !workspace) {
      return;
    }

    if (event.target instanceof HTMLElement && event.target.closest("button")) {
      return;
    }

    const panelRect = panel.getBoundingClientRect();
    dragStateRef.current = {
      pointerId: event.pointerId,
      offsetX: event.clientX - panelRect.left,
      offsetY: event.clientY - panelRect.top
    };
  };

  return {
    panelRef,
    handleHeaderPointerDown,
    panelStyle: position
      ? { left: position.x, top: position.y, right: "auto" }
      : undefined
  };
}
