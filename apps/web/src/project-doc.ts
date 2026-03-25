import { getAreaSqFt } from "./units";

export const DEFAULT_STORY_HEIGHT_FT = 10;

const LEVEL_ID_PATTERN = /^level-(\d+)$/i;
const LEVEL_NAME_PATTERN = /^Level (\d+)$/;

export type ProjectDoc = {
  projectId: string;
  name: string;
  units: "imperial-ft-in";
  defaultStoryHeightFt: number;
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

export type AutoGenerateLevelsInput = {
  storiesBelowGrade: number;
  storiesOnGrade: number;
  storyHeightFt: number;
};

export type LevelMutationResult = {
  doc: ProjectDoc;
  activeLevelId: string;
};

function getPositiveFeetOrFallback(value: number, fallback: number): number {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function getStoryCount(value: number): number {
  return Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
}

function createLevelIdFactory(levels: Level[]): () => string {
  let nextNumber = levels.reduce((maxNumber, level) => {
    const match = LEVEL_ID_PATTERN.exec(level.id);
    return match ? Math.max(maxNumber, Number(match[1])) : maxNumber;
  }, 0);

  return () => {
    nextNumber += 1;
    return `level-${String(nextNumber).padStart(2, "0")}`;
  };
}

function getNextManualLevelName(levels: Level[]): string {
  const nextNumber = levels.reduce((maxNumber, level) => {
    const match = LEVEL_NAME_PATTERN.exec(level.name);
    return match ? Math.max(maxNumber, Number(match[1])) : maxNumber;
  }, 0) + 1;

  return `Level ${nextNumber}`;
}

function swapItems<T>(items: readonly T[], leftIndex: number, rightIndex: number): T[] {
  const nextItems = [...items];
  const leftValue = nextItems[leftIndex];
  nextItems[leftIndex] = nextItems[rightIndex];
  nextItems[rightIndex] = leftValue;
  return nextItems;
}

function buildGeneratedLevelSpecs(input: AutoGenerateLevelsInput): Array<Pick<Level, "name" | "elevationFt" | "heightFt">> {
  const storiesBelowGrade = getStoryCount(input.storiesBelowGrade);
  const storiesOnGrade = getStoryCount(input.storiesOnGrade);
  const storyHeightFt = getPositiveFeetOrFallback(input.storyHeightFt, DEFAULT_STORY_HEIGHT_FT);
  const levels: Array<Pick<Level, "name" | "elevationFt" | "heightFt">> = [];

  for (let basementIndex = storiesBelowGrade; basementIndex >= 1; basementIndex -= 1) {
    levels.push({
      name: `Basement ${basementIndex}`,
      elevationFt: -basementIndex * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  for (let levelNumber = 1; levelNumber <= storiesOnGrade; levelNumber += 1) {
    levels.push({
      name: `Level ${levelNumber}`,
      elevationFt: (levelNumber - 1) * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  return levels;
}

function getPreferredGeneratedLevelName(levels: Array<Pick<Level, "name">>): string {
  return levels.find((level) => level.name === "Level 1")?.name
    ?? levels.at(-1)?.name
    ?? "Level 1";
}

export function createStarterProjectDoc(): ProjectDoc {
  return {
    projectId: "project-starter",
    name: "GLOQ Starter Floor",
    units: "imperial-ft-in",
    defaultStoryHeightFt: DEFAULT_STORY_HEIGHT_FT,
    levels: [
      {
        id: "level-01",
        name: "Level 1",
        elevationFt: 0,
        heightFt: DEFAULT_STORY_HEIGHT_FT
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

export function getLevelById(doc: ProjectDoc, levelId: string | null | undefined): Level | null {
  if (!levelId) {
    return null;
  }

  return doc.levels.find((level) => level.id === levelId) ?? null;
}

export function getValidActiveLevelId(doc: ProjectDoc, activeLevelId: string | null | undefined): string {
  return getLevelById(doc, activeLevelId)?.id ?? doc.levels[0]?.id ?? "";
}

export function getLevelSpaces(doc: ProjectDoc, levelId: string): Space[] {
  return doc.spaces.filter((space) => space.levelId === levelId);
}

export function getSpaceAreaSqFt(space: Space): number {
  return getAreaSqFt(space.widthFt, space.depthFt);
}

export function createLevel(doc: ProjectDoc, activeLevelId: string | null | undefined): LevelMutationResult {
  const idFactory = createLevelIdFactory(doc.levels);
  const insertionAnchor = getLevelById(doc, activeLevelId) ?? doc.levels.at(-1);
  const insertionIndex = insertionAnchor
    ? doc.levels.findIndex((level) => level.id === insertionAnchor.id) + 1
    : doc.levels.length;
  const newLevel: Level = {
    id: idFactory(),
    name: getNextManualLevelName(doc.levels),
    elevationFt: insertionAnchor ? insertionAnchor.elevationFt + insertionAnchor.heightFt : 0,
    heightFt: doc.defaultStoryHeightFt
  };

  return {
    doc: {
      ...doc,
      levels: [
        ...doc.levels.slice(0, insertionIndex),
        newLevel,
        ...doc.levels.slice(insertionIndex)
      ]
    },
    activeLevelId: newLevel.id
  };
}

export function renameLevel(doc: ProjectDoc, levelId: string, name: string): ProjectDoc {
  return {
    ...doc,
    levels: doc.levels.map((level) => (
      level.id === levelId
        ? { ...level, name }
        : level
    ))
  };
}

export function setLevelElevation(doc: ProjectDoc, levelId: string, elevationFt: number): ProjectDoc {
  if (!Number.isFinite(elevationFt)) {
    return doc;
  }

  return {
    ...doc,
    levels: doc.levels.map((level) => (
      level.id === levelId
        ? { ...level, elevationFt }
        : level
    ))
  };
}

export function setDefaultStoryHeight(doc: ProjectDoc, defaultStoryHeightFt: number): ProjectDoc {
  return {
    ...doc,
    defaultStoryHeightFt: getPositiveFeetOrFallback(defaultStoryHeightFt, doc.defaultStoryHeightFt)
  };
}

export function deleteLevel(
  doc: ProjectDoc,
  levelId: string,
  activeLevelId: string | null | undefined
): LevelMutationResult {
  const currentIndex = doc.levels.findIndex((level) => level.id === levelId);

  if (currentIndex === -1 || doc.levels.length <= 1) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, activeLevelId)
    };
  }

  const levels = doc.levels.filter((level) => level.id !== levelId);
  const spaces = doc.spaces.filter((space) => space.levelId !== levelId);
  const fallbackIndex = Math.min(currentIndex, levels.length - 1);
  const nextActiveLevelId = activeLevelId === levelId
    ? levels[fallbackIndex]?.id ?? levels[0].id
    : getValidActiveLevelId({ ...doc, levels, spaces }, activeLevelId);

  return {
    doc: {
      ...doc,
      levels,
      spaces
    },
    activeLevelId: nextActiveLevelId
  };
}

export function moveLevel(doc: ProjectDoc, levelId: string, direction: "up" | "down"): ProjectDoc {
  const currentIndex = doc.levels.findIndex((level) => level.id === levelId);

  if (currentIndex === -1) {
    return doc;
  }

  const nextIndex = direction === "up" ? currentIndex - 1 : currentIndex + 1;

  if (nextIndex < 0 || nextIndex >= doc.levels.length) {
    return doc;
  }

  return {
    ...doc,
    levels: swapItems(doc.levels, currentIndex, nextIndex)
  };
}

export function autoGenerateLevels(doc: ProjectDoc, input: AutoGenerateLevelsInput): LevelMutationResult {
  const specs = buildGeneratedLevelSpecs(input);

  if (specs.length === 0) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, doc.levels[0]?.id)
    };
  }

  const idFactory = createLevelIdFactory(doc.levels);
  const reusedLevelIds = new Set<string>();
  const levels = specs.map((spec) => {
    const reusableLevel = doc.levels.find((level) => (
      level.name === spec.name
      && !reusedLevelIds.has(level.id)
    ));
    const nextLevelId = reusableLevel?.id ?? idFactory();

    reusedLevelIds.add(nextLevelId);

    return {
      id: nextLevelId,
      ...spec
    };
  });
  const survivingLevelIds = new Set(levels.map((level) => level.id));
  const activeLevelName = getPreferredGeneratedLevelName(specs);
  const activeLevelId = levels.find((level) => level.name === activeLevelName)?.id ?? levels[0].id;

  return {
    doc: {
      ...doc,
      defaultStoryHeightFt: getPositiveFeetOrFallback(input.storyHeightFt, doc.defaultStoryHeightFt),
      levels,
      spaces: doc.spaces.filter((space) => survivingLevelIds.has(space.levelId))
    },
    activeLevelId
  };
}
