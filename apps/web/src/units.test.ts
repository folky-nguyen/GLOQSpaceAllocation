import { describe, expect, it } from "vitest";
import {
  feetToMeters,
  formatFeetAndInches,
  getAreaSqFt,
  metersToFeet,
  parseFeetAndInches
} from "./units";

const ROUND_TRIP_TOLERANCE_FT = 1 / (12 * 32);

function expectFeetCloseTo(actual: number | null, expected: number) {
  expect(actual).not.toBeNull();
  expect(actual as number).toBeCloseTo(expected, 8);
}

describe("parseFeetAndInches", () => {
  it("parses the required supported samples", () => {
    expectFeetCloseTo(parseFeetAndInches("12'"), 12);
    expectFeetCloseTo(parseFeetAndInches("12' 6\""), 12.5);
    expectFeetCloseTo(parseFeetAndInches("12'-6 1/2\""), 12 + 6.5 / 12);
    expectFeetCloseTo(parseFeetAndInches("7 1/4\""), 7.25 / 12);
    expectFeetCloseTo(parseFeetAndInches("9.5'"), 9.5);
    expectFeetCloseTo(parseFeetAndInches("-1' 6\""), -1.5);
  });

  it("accepts inch overflow and normalizes through total inches", () => {
    expectFeetCloseTo(parseFeetAndInches("14\""), 14 / 12);
    expectFeetCloseTo(parseFeetAndInches("5' 14\""), 6 + 2 / 12);
  });

  it("rejects invalid inputs", () => {
    expect(parseFeetAndInches("")).toBeNull();
    expect(parseFeetAndInches("12")).toBeNull();
    expect(parseFeetAndInches("abc")).toBeNull();
    expect(parseFeetAndInches("1' 2/0\"")).toBeNull();
    expect(parseFeetAndInches("1' -2\"")).toBeNull();
    expect(parseFeetAndInches("1 1/2'")).toBeNull();
    expect(parseFeetAndInches("\"")).toBeNull();
  });
});

describe("formatFeetAndInches", () => {
  it("formats normalized imperial strings with sensible defaults", () => {
    expect(formatFeetAndInches(12)).toBe("12'");
    expect(formatFeetAndInches(12.5)).toBe("12' 6\"");
    expect(formatFeetAndInches(12 + 6.5 / 12)).toBe("12' 6 1/2\"");
    expect(formatFeetAndInches(7.25 / 12)).toBe("7 1/4\"");
    expect(formatFeetAndInches(0)).toBe("0\"");
    expect(formatFeetAndInches(-1.5)).toBe("-1' 6\"");
  });

  it("supports denominator changes and inch overflow carry", () => {
    expect(formatFeetAndInches(7.25 / 12, { inchDenominator: 4 })).toBe("7 1/4\"");
    expect(formatFeetAndInches(11 + 11.96875 / 12)).toBe("12'");
  });
});

describe("conversion and area helpers", () => {
  it("converts between feet and meters using international feet", () => {
    expect(feetToMeters(1)).toBe(0.3048);
    expect(metersToFeet(0.3048)).toBe(1);

    const representativeValue = 3.14159;
    expect(feetToMeters(metersToFeet(representativeValue))).toBeCloseTo(representativeValue, 10);
  });

  it("returns rectangular area in square feet", () => {
    expect(getAreaSqFt(24, 18)).toBe(432);
  });
});

describe("round-trip behavior", () => {
  it("round-trips supported samples through normalized formatting", () => {
    const samples = [
      "12'",
      "12' 6\"",
      "12'-6 1/2\"",
      "7 1/4\"",
      "9.5'",
      "-1' 6\""
    ];

    for (const sample of samples) {
      const parsed = parseFeetAndInches(sample);
      expect(parsed).not.toBeNull();

      const formatted = formatFeetAndInches(parsed as number);
      const reparsed = parseFeetAndInches(formatted);

      expect(reparsed).not.toBeNull();
      expect(Math.abs((reparsed as number) - (parsed as number))).toBeLessThanOrEqual(ROUND_TRIP_TOLERANCE_FT);
    }
  });

  it("round-trips representative decimal feet values within display precision", () => {
    const lengthsFt = [
      0,
      0.125,
      0.5,
      1.125,
      12.0416666667,
      24.9875,
      -3.2083333333
    ];

    for (const lengthFt of lengthsFt) {
      const formatted = formatFeetAndInches(lengthFt);
      const parsed = parseFeetAndInches(formatted);

      expect(parsed).not.toBeNull();
      expect(Math.abs((parsed as number) - lengthFt)).toBeLessThanOrEqual(ROUND_TRIP_TOLERANCE_FT);
    }
  });
});
