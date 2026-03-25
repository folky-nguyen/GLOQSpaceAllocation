export const METERS_PER_FOOT = 0.3048;

const DEFAULT_INCH_DENOMINATOR = 16;
const FEET_SUBDIVISIONS_PER_FOOT = 12;
const FEET_ONLY_PATTERN = /^(\d+(?:\.\d+)?)'$/;
const FEET_AND_INCHES_PATTERNS = [
  /^(\d+(?:\.\d+)?)'\s+-\s+(.+)"$/,
  /^(\d+(?:\.\d+)?)'-(.+)"$/,
  /^(\d+(?:\.\d+)?)'\s+(.+)"$/
] as const;
const INCHES_ONLY_PATTERN = /^(.+)"$/;

export type InchDenominator = 2 | 4 | 8 | 16;

function parseUnsignedDecimal(text: string): number | null {
  if (!/^\d+(?:\.\d+)?$/.test(text)) {
    return null;
  }

  return Number(text);
}

function parseFraction(text: string): number | null {
  const match = /^(\d+)\/(\d+)$/.exec(text);

  if (!match) {
    return null;
  }

  const numerator = Number(match[1]);
  const denominator = Number(match[2]);

  if (denominator === 0) {
    return null;
  }

  return numerator / denominator;
}

function parseInchesComponent(text: string): number | null {
  const value = text.trim();

  if (!value || value.startsWith("-") || value.startsWith("+")) {
    return null;
  }

  const wholeAndFractionMatch = /^(\d+)\s+(\d+)\/(\d+)$/.exec(value);

  if (wholeAndFractionMatch) {
    const wholeInches = Number(wholeAndFractionMatch[1]);
    const fraction = parseFraction(`${wholeAndFractionMatch[2]}/${wholeAndFractionMatch[3]}`);
    return fraction === null ? null : wholeInches + fraction;
  }

  const fraction = parseFraction(value);

  if (fraction !== null) {
    return fraction;
  }

  if (/^\d+$/.test(value)) {
    return Number(value);
  }

  return null;
}

function normalizeSignedInput(input: string): { sign: 1 | -1; body: string } | null {
  const trimmed = input.trim();

  if (!trimmed) {
    return null;
  }

  if (trimmed.startsWith("-")) {
    const body = trimmed.slice(1).trimStart();
    return body ? { sign: -1, body } : null;
  }

  if (trimmed.startsWith("+")) {
    return null;
  }

  return { sign: 1, body: trimmed };
}

function parseFeetAndInchesParts(body: string): { feet: number; inches: number } | null {
  const feetOnlyMatch = FEET_ONLY_PATTERN.exec(body);

  if (feetOnlyMatch) {
    const feet = parseUnsignedDecimal(feetOnlyMatch[1]);
    return feet === null ? null : { feet, inches: 0 };
  }

  for (const pattern of FEET_AND_INCHES_PATTERNS) {
    const match = pattern.exec(body);

    if (!match) {
      continue;
    }

    const feet = parseUnsignedDecimal(match[1]);
    const inches = parseInchesComponent(match[2]);

    if (feet === null || inches === null) {
      return null;
    }

    return { feet, inches };
  }

  const inchesOnlyMatch = INCHES_ONLY_PATTERN.exec(body);

  if (!inchesOnlyMatch) {
    return null;
  }

  const inches = parseInchesComponent(inchesOnlyMatch[1]);
  return inches === null ? null : { feet: 0, inches };
}

function getGreatestCommonDivisor(left: number, right: number): number {
  let a = Math.abs(left);
  let b = Math.abs(right);

  while (b !== 0) {
    const remainder = a % b;
    a = b;
    b = remainder;
  }

  return a || 1;
}

function getInchDenominator(denominator: InchDenominator | undefined): InchDenominator {
  return denominator ?? DEFAULT_INCH_DENOMINATOR;
}

function toMixedInchesParts(totalSubdivisionInches: number, denominator: InchDenominator) {
  const subdivisionsPerFoot = FEET_SUBDIVISIONS_PER_FOOT * denominator;
  const feet = Math.floor(totalSubdivisionInches / subdivisionsPerFoot);
  const inchesSubdivisions = totalSubdivisionInches - feet * subdivisionsPerFoot;
  const wholeInches = Math.floor(inchesSubdivisions / denominator);
  const fractionNumerator = inchesSubdivisions - wholeInches * denominator;

  if (fractionNumerator === 0) {
    return {
      feet,
      wholeInches,
      fractionNumerator: 0,
      fractionDenominator: denominator
    };
  }

  const divisor = getGreatestCommonDivisor(fractionNumerator, denominator);

  return {
    feet,
    wholeInches,
    fractionNumerator: fractionNumerator / divisor,
    fractionDenominator: denominator / divisor
  };
}

function roundInchesToDenominator(totalInches: number, denominator: InchDenominator): number {
  return Math.round(totalInches * denominator);
}

export function feetToMeters(feet: number): number {
  return feet * METERS_PER_FOOT;
}

export function metersToFeet(meters: number): number {
  return meters / METERS_PER_FOOT;
}

export function getAreaSqFt(widthFt: number, depthFt: number): number {
  return widthFt * depthFt;
}

export function parseFeetAndInches(input: string): number | null {
  const normalizedInput = normalizeSignedInput(input);

  if (!normalizedInput) {
    return null;
  }

  const parts = parseFeetAndInchesParts(normalizedInput.body);

  if (!parts) {
    return null;
  }

  return normalizedInput.sign * (parts.feet + parts.inches / FEET_SUBDIVISIONS_PER_FOOT);
}

export function formatFeetAndInches(
  lengthFt: number,
  options?: { inchDenominator?: InchDenominator }
): string {
  const denominator = getInchDenominator(options?.inchDenominator);
  const totalSubdivisionInches = roundInchesToDenominator(
    Math.abs(lengthFt) * FEET_SUBDIVISIONS_PER_FOOT,
    denominator
  );

  if (totalSubdivisionInches === 0) {
    return "0\"";
  }

  const sign = lengthFt < 0 ? "-" : "";
  const {
    feet,
    wholeInches,
    fractionNumerator,
    fractionDenominator
  } = toMixedInchesParts(totalSubdivisionInches, denominator);

  const fraction = fractionNumerator === 0
    ? ""
    : `${fractionNumerator}/${fractionDenominator}`;

  if (feet === 0) {
    if (!fraction) {
      return `${sign}${wholeInches}"`;
    }

    if (wholeInches === 0) {
      return `${sign}${fraction}"`;
    }

    return `${sign}${wholeInches} ${fraction}"`;
  }

  if (wholeInches === 0 && !fraction) {
    return `${sign}${feet}'`;
  }

  if (!fraction) {
    return `${sign}${feet}' ${wholeInches}"`;
  }

  if (wholeInches === 0) {
    return `${sign}${feet}' ${fraction}"`;
  }

  return `${sign}${feet}' ${wholeInches} ${fraction}"`;
}
