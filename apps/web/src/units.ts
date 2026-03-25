export const METERS_PER_FOOT = 0.3048;

const DEFAULT_INCH_DENOMINATOR = 16;
const INCHES_PER_FOOT = 12;
const FEET_ONLY_PATTERN = /^(\d+(?:\.\d+)?)'$/;
const EXPLICIT_FEET_PATTERN = /^(\d+(?:\.\d+)?)'(.*)$/;
const INCHES_ONLY_PATTERN = /^(.+)"$/;
const INTEGER_PATTERN = /^\d+$/;
const DECIMAL_PATTERN = /^\d+(?:\.\d+)?$/;
const FRACTION_PATTERN = /^(\d+)\/(\d+)$/;
const SINGLE_QUOTE_NORMALIZER = /[\u2018\u2019\u2032\u2035]/g;
const DOUBLE_QUOTE_NORMALIZER = /[\u201C\u201D\u2033\u2036]/g;

export type InchDenominator = 2 | 4 | 8 | 16;
export type DefaultLengthUnit = "ft" | "in";

type LengthParts = {
  feet: number;
  inches: number;
};

function parseIntegerToken(text: string): number | null {
  return INTEGER_PATTERN.test(text) ? Number(text) : null;
}

function parseUnsignedDecimal(text: string): number | null {
  return DECIMAL_PATTERN.test(text) ? Number(text) : null;
}

function parseFractionToken(text: string): number | null {
  const match = FRACTION_PATTERN.exec(text);

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

  const tokens = value.split(" ");

  if (tokens.length === 1) {
    const fraction = parseFractionToken(tokens[0]);

    if (fraction !== null) {
      return fraction;
    }

    return parseUnsignedDecimal(tokens[0]);
  }

  if (tokens.length !== 2) {
    return null;
  }

  const wholeInches = parseIntegerToken(tokens[0]);
  const fraction = parseFractionToken(tokens[1]);

  if (wholeInches === null || fraction === null) {
    return null;
  }

  return wholeInches + fraction;
}

function normalizeImperialInput(input: string): string {
  return input
    .trim()
    .replace(SINGLE_QUOTE_NORMALIZER, "'")
    .replace(DOUBLE_QUOTE_NORMALIZER, "\"")
    .replace(/''/g, "\"")
    .replace(/\s+/g, " ");
}

function normalizeSignedInput(input: string): { sign: 1 | -1; body: string } | null {
  const normalized = normalizeImperialInput(input);

  if (!normalized) {
    return null;
  }

  if (normalized.startsWith("-")) {
    const body = normalized.slice(1).trimStart();
    return body ? { sign: -1, body } : null;
  }

  if (normalized.startsWith("+")) {
    return null;
  }

  return { sign: 1, body: normalized };
}

function parseExplicitImperial(body: string): LengthParts | null {
  const feetOnlyMatch = FEET_ONLY_PATTERN.exec(body);

  if (feetOnlyMatch) {
    const feet = parseUnsignedDecimal(feetOnlyMatch[1]);
    return feet === null ? null : { feet, inches: 0 };
  }

  const explicitFeetMatch = EXPLICIT_FEET_PATTERN.exec(body);

  if (explicitFeetMatch) {
    const feet = parseUnsignedDecimal(explicitFeetMatch[1]);

    if (feet === null) {
      return null;
    }

    const remainder = explicitFeetMatch[2];

    if (!remainder.trim()) {
      return { feet, inches: 0 };
    }

    const explicitInchesMatch = /^\s*-?\s*(.+)"$/.exec(remainder);

    if (!explicitInchesMatch) {
      return null;
    }

    const inches = parseInchesComponent(explicitInchesMatch[1]);
    return inches === null ? null : { feet, inches };
  }

  const inchesOnlyMatch = INCHES_ONLY_PATTERN.exec(body);

  if (!inchesOnlyMatch) {
    return null;
  }

  const inches = parseInchesComponent(inchesOnlyMatch[1]);
  return inches === null ? null : { feet: 0, inches };
}

function tokenizeMarkerlessShorthand(body: string): string[] | null {
  if (!body) {
    return null;
  }

  let candidate = body;

  if (candidate.includes("'")) {
    return null;
  }

  if (candidate.endsWith("\"")) {
    candidate = candidate.slice(0, -1).trimEnd();
  }

  if (!candidate || candidate.includes("\"")) {
    return null;
  }

  return candidate.split(" ");
}

function parseMarkerlessShorthand(body: string): LengthParts | null {
  const tokens = tokenizeMarkerlessShorthand(body);

  if (!tokens || tokens.length < 2 || tokens.length > 3) {
    return null;
  }

  if (tokens.length === 2) {
    const leftInteger = parseIntegerToken(tokens[0]);

    if (leftInteger === null) {
      return null;
    }

    const rightInteger = parseIntegerToken(tokens[1]);

    if (rightInteger !== null) {
      return { feet: leftInteger, inches: rightInteger };
    }

    const fraction = parseFractionToken(tokens[1]);

    if (fraction !== null) {
      return { feet: 0, inches: leftInteger + fraction };
    }

    return null;
  }

  const feet = parseIntegerToken(tokens[0]);
  const wholeInches = parseIntegerToken(tokens[1]);
  const fraction = parseFractionToken(tokens[2]);

  if (feet === null || wholeInches === null || fraction === null) {
    return null;
  }

  return { feet, inches: wholeInches + fraction };
}

function parseBareLengthToken(body: string): number | null {
  if (!body || body.includes("'") || body.includes("\"") || body.includes(" ")) {
    return null;
  }

  return parseUnsignedDecimal(body);
}

function toFeetFromDefaultUnit(value: number, defaultUnit: DefaultLengthUnit): number {
  return defaultUnit === "in" ? value / INCHES_PER_FOOT : value;
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
  const subdivisionsPerFoot = INCHES_PER_FOOT * denominator;
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

export function parseFeetAndInches(
  input: string,
  options?: { defaultUnit?: DefaultLengthUnit }
): number | null {
  const normalizedInput = normalizeSignedInput(input);

  if (!normalizedInput) {
    return null;
  }

  const defaultUnit = options?.defaultUnit ?? "ft";
  const parts = parseExplicitImperial(normalizedInput.body)
    ?? parseMarkerlessShorthand(normalizedInput.body);

  if (parts) {
    return normalizedInput.sign * (parts.feet + parts.inches / INCHES_PER_FOOT);
  }

  const bareValue = parseBareLengthToken(normalizedInput.body);

  if (bareValue === null) {
    return null;
  }

  return normalizedInput.sign * toFeetFromDefaultUnit(bareValue, defaultUnit);
}

export function formatFeetAndInches(
  lengthFt: number,
  options?: { inchDenominator?: InchDenominator }
): string {
  const denominator = getInchDenominator(options?.inchDenominator);
  const totalSubdivisionInches = roundInchesToDenominator(
    Math.abs(lengthFt) * INCHES_PER_FOOT,
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
