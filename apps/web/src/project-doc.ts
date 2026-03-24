export type ProjectDoc = {
  projectId: string;
  name: string;
  units: "imperial-ft-in";
  levels: Level[];
  spaces: Space[];
};

export type Level = {
  id: string;
  name: string;
  elevationFt: number;
  heightFt: number;
};

export type Space = {
  id: string;
  levelId: string;
  name: string;
  xFt: number;
  yFt: number;
  widthFt: number;
  depthFt: number;
};

export function createStarterProjectDoc(): ProjectDoc {
  return {
    projectId: "project-starter",
    name: "GLOQ Starter Floor",
    units: "imperial-ft-in",
    levels: [
      {
        id: "level-01",
        name: "Level 1",
        elevationFt: 0,
        heightFt: 12
      }
    ],
    spaces: [
      {
        id: "space-lobby",
        levelId: "level-01",
        name: "Lobby",
        xFt: 0,
        yFt: 0,
        widthFt: 24,
        depthFt: 18
      },
      {
        id: "space-studio",
        levelId: "level-01",
        name: "Studio",
        xFt: 24,
        yFt: 0,
        widthFt: 32,
        depthFt: 24
      }
    ]
  };
}

export function formatFeetAndInches(lengthFt: number): string {
  const totalInches = Math.round(lengthFt * 12);
  const feet = Math.floor(totalInches / 12);
  const inches = totalInches % 12;
  return `${feet}' ${inches}"`;
}

export function getLevelSpaces(doc: ProjectDoc, levelId: string): Space[] {
  return doc.spaces.filter((space) => space.levelId === levelId);
}

export function getSpaceAreaSqFt(space: Space): number {
  return space.widthFt * space.depthFt;
}

