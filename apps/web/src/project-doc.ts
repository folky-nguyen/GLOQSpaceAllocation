import { getAreaSqFt } from "./units";

export const DEFAULT_STORY_HEIGHT_FT = 10;

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

export type ProjectDoc = {
  id: string;
  name: string;
  defaultStoryHeightFt: number;
  levels: Level[];
  spaces: Space[];
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

type GeneratedLevelSpec = {
  name: string;
  elevationFt: number;
  heightFt: number;
};

function getPositiveFeetOrFallback(value: number, fallback: number): number {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function getStoryCount(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.floor(value));
}

function createLevelIdFactory(levels: Level[]): () => string {
  const usedIds = new Set(levels.map((level) => level.id));
  let nextId = levels.reduce((highestId, level) => {
    const match = /^level-(\d+)$/.exec(level.id);
    return match ? Math.max(highestId, Number(match[1])) : highestId;
  }, 0) + 1;

  return () => {
    while (usedIds.has(`level-${nextId}`)) {
      nextId += 1;
    }

    const id = `level-${nextId}`;
    usedIds.add(id);
    nextId += 1;
    return id;
  };
}

function getNextManualLevelName(levels: Level[]): string {
  const usedNames = new Set(levels.map((level) => level.name));
  let nextIndex = 1;

  while (usedNames.has(`Level ${nextIndex}`)) {
    nextIndex += 1;
  }

  return `Level ${nextIndex}`;
}

function swapItems<T>(items: T[], leftIndex: number, rightIndex: number): T[] {
  const nextItems = [...items];
  const leftItem = nextItems[leftIndex];

  nextItems[leftIndex] = nextItems[rightIndex];
  nextItems[rightIndex] = leftItem;

  return nextItems;
}

function buildGeneratedLevelSpecs(input: AutoGenerateLevelsInput): GeneratedLevelSpec[] {
  const storiesBelowGrade = getStoryCount(input.storiesBelowGrade);
  const storiesOnGrade = getStoryCount(input.storiesOnGrade);
  const storyHeightFt = getPositiveFeetOrFallback(input.storyHeightFt, DEFAULT_STORY_HEIGHT_FT);
  const levels: GeneratedLevelSpec[] = [];

  for (let story = storiesBelowGrade; story >= 1; story -= 1) {
    levels.push({
      name: `Basement ${story}`,
      elevationFt: -story * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  for (let story = 1; story <= storiesOnGrade; story += 1) {
    levels.push({
      name: `Level ${story}`,
      elevationFt: (story - 1) * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  if (levels.length === 0) {
    levels.push({
      name: "Level 1",
      elevationFt: 0,
      heightFt: storyHeightFt
    });
  }

  return levels;
}

function getPreferredGeneratedLevelName(levels: GeneratedLevelSpec[]): string {
  return levels.find((level) => level.name === "Level 1")?.name
    ?? levels.at(-1)?.name
    ?? "Level 1";
}

export function createStarterProjectDoc(): ProjectDoc {
  const levelId = "level-1";

  return {
    id: "project-starter",
    name: "GLOQ Tower",
    defaultStoryHeightFt: DEFAULT_STORY_HEIGHT_FT,
    levels: [
      {
        id: levelId,
        name: "Level 1",
        elevationFt: 0,
        heightFt: DEFAULT_STORY_HEIGHT_FT
      }
    ],
    spaces: [
      {
        id: "space-lobby",
        levelId,
        name: "Lobby",
        xFt: 0,
        yFt: 0,
        widthFt: 18,
        depthFt: 14
      },
      {
        id: "space-conference",
        levelId,
        name: "Conference",
        xFt: 20,
        yFt: 0,
        widthFt: 16,
        depthFt: 12
      },
      {
        id: "space-open-office",
        levelId,
        name: "Open Office",
        xFt: 0,
        yFt: 16,
        widthFt: 24,
        depthFt: 18
      }
    ]
  };
}

export function getLevelById(doc: ProjectDoc, levelId: string): Level | null {
  return doc.levels.find((level) => level.id === levelId) ?? null;
}

export function getValidActiveLevelId(doc: ProjectDoc, activeLevelId: string | null | undefined): string {
  if (activeLevelId && doc.levels.some((level) => level.id === activeLevelId)) {
    return activeLevelId;
  }

  return doc.levels[0]?.id ?? "";
}

export function getLevelSpaces(doc: ProjectDoc, levelId: string): Space[] {
  return doc.spaces.filter((space) => space.levelId === levelId);
}

export function getSpaceAreaSqFt(space: Space): number {
  return getAreaSqFt(space.widthFt, space.depthFt);
}

export function createLevel(doc: ProjectDoc, activeLevelId: string): LevelMutationResult {
  const insertAfterId = getValidActiveLevelId(doc, activeLevelId);
  const insertAfterIndex = Math.max(0, doc.levels.findIndex((level) => level.id === insertAfterId));
  const insertAfterLevel = doc.levels[insertAfterIndex] ?? doc.levels.at(-1);

  if (!insertAfterLevel) {
    return {
      doc,
      activeLevelId: ""
    };
  }

  const nextId = createLevelIdFactory(doc.levels);
  const heightFt = getPositiveFeetOrFallback(doc.defaultStoryHeightFt, DEFAULT_STORY_HEIGHT_FT);
  const level: Level = {
    id: nextId(),
    name: getNextManualLevelName(doc.levels),
    elevationFt: insertAfterLevel.elevationFt + heightFt,
    heightFt
  };

  return {
    doc: {
      ...doc,
      levels: [
        ...doc.levels.slice(0, insertAfterIndex + 1),
        level,
        ...doc.levels.slice(insertAfterIndex + 1)
      ]
    },
    activeLevelId: level.id
  };
}

export function deleteLevel(doc: ProjectDoc, levelId: string, activeLevelId: string): LevelMutationResult {
  if (doc.levels.length <= 1) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, activeLevelId)
    };
  }

  const deleteIndex = doc.levels.findIndex((level) => level.id === levelId);

  if (deleteIndex === -1) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, activeLevelId)
    };
  }

  const levels = doc.levels.filter((level) => level.id !== levelId);
  const nextDoc: ProjectDoc = {
    ...doc,
    levels,
    spaces: doc.spaces.filter((space) => space.levelId !== levelId)
  };
  const fallbackLevel = levels[Math.min(deleteIndex, levels.length - 1)] ?? levels[0];

  return {
    doc: nextDoc,
    activeLevelId: activeLevelId === levelId
      ? fallbackLevel.id
      : getValidActiveLevelId(nextDoc, activeLevelId)
  };
}

export function renameLevel(doc: ProjectDoc, levelId: string, name: string): ProjectDoc {
  const trimmedName = name.trim();

  if (!trimmedName) {
    return doc;
  }

  return {
    ...doc,
    levels: doc.levels.map((level) => (
      level.id === levelId
        ? { ...level, name: trimmedName }
        : level
    ))
  };
}

export function moveLevel(doc: ProjectDoc, levelId: string, direction: "up" | "down"): ProjectDoc {
  const index = doc.levels.findIndex((level) => level.id === levelId);

  if (index === -1) {
    return doc;
  }

  const nextIndex = direction === "up" ? index - 1 : index + 1;

  if (nextIndex < 0 || nextIndex >= doc.levels.length) {
    return doc;
  }

  return {
    ...doc,
    levels: swapItems(doc.levels, index, nextIndex)
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

export function setDefaultStoryHeight(doc: ProjectDoc, heightFt: number): ProjectDoc {
  if (!Number.isFinite(heightFt) || heightFt <= 0) {
    return doc;
  }

  return {
    ...doc,
    defaultStoryHeightFt: heightFt
  };
}

export function autoGenerateLevels(doc: ProjectDoc, input: AutoGenerateLevelsInput): LevelMutationResult {
  const levelsToGenerate = buildGeneratedLevelSpecs(input);
  const storyHeightFt = levelsToGenerate[0]?.heightFt ?? DEFAULT_STORY_HEIGHT_FT;
  const existingLevelsByName = new Map<string, Level[]>();

  for (const level of doc.levels) {
    const matchingLevels = existingLevelsByName.get(level.name) ?? [];
    matchingLevels.push(level);
    existingLevelsByName.set(level.name, matchingLevels);
  }

  const nextId = createLevelIdFactory(doc.levels);
  const levels = levelsToGenerate.map((levelSpec) => {
    const reusableLevel = existingLevelsByName.get(levelSpec.name)?.shift();

    return {
      id: reusableLevel?.id ?? nextId(),
      name: levelSpec.name,
      elevationFt: levelSpec.elevationFt,
      heightFt: levelSpec.heightFt
    };
  });
  const keptLevelIds = new Set(levels.map((level) => level.id));
  const nextDoc: ProjectDoc = {
    ...doc,
    defaultStoryHeightFt: storyHeightFt,
    levels,
    spaces: doc.spaces.filter((space) => keptLevelIds.has(space.levelId))
  };
  const preferredLevelName = getPreferredGeneratedLevelName(levelsToGenerate);
  const preferredLevel = levels.find((level) => level.name === preferredLevelName) ?? levels[0];

  return {
    doc: nextDoc,
    activeLevelId: preferredLevel.id
  };
}
