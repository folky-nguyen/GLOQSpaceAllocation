import { useState } from "react";
import {
  feetToMeters,
  formatFeetAndInches,
  getAreaSqFt,
  metersToFeet,
  parseFeetAndInches,
  type DefaultLengthUnit,
  type InchDenominator
} from "./units";

type UnitsInspectorProps = {
  open: boolean;
  onClose: () => void;
};

const sampleInputs = [
  "1.24",
  "1.2\"",
  "1.2''",
  "12'",
  "12'6\"",
  "12' 6\"",
  "12'-6 1/2\"",
  "12 3 3/4",
  "7''",
  "7 1/4\"",
  "9.5'"
] as const;

const denominatorOptions: InchDenominator[] = [2, 4, 8, 16];

function parseFiniteNumber(value: string): number | null {
  const trimmed = value.trim();

  if (!trimmed) {
    return null;
  }

  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

function formatNumber(value: number, maximumFractionDigits = 6): string {
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits,
    minimumFractionDigits: 0
  }).format(value);
}

export default function UnitsInspector({ open, onClose }: UnitsInspectorProps) {
  const [parseInput, setParseInput] = useState("1.24");
  const [defaultUnit, setDefaultUnit] = useState<DefaultLengthUnit>("ft");
  const [feetInput, setFeetInput] = useState("12.5");
  const [inchDenominator, setInchDenominator] = useState<InchDenominator>(16);
  const [metersInput, setMetersInput] = useState("3.048");
  const [widthInput, setWidthInput] = useState("24");
  const [depthInput, setDepthInput] = useState("18");

  if (!open) {
    return null;
  }

  const parsedFeet = parseFeetAndInches(parseInput, { defaultUnit });
  const parsedMeters = parsedFeet === null ? null : feetToMeters(parsedFeet);
  const parsedNormalized = parsedFeet === null ? null : formatFeetAndInches(parsedFeet, { inchDenominator });

  const feetValue = parseFiniteNumber(feetInput);
  const formattedFeet = feetValue === null ? null : formatFeetAndInches(feetValue, { inchDenominator });
  const formattedFeetMeters = feetValue === null ? null : feetToMeters(feetValue);

  const metersValue = parseFiniteNumber(metersInput);
  const convertedFeet = metersValue === null ? null : metersToFeet(metersValue);
  const convertedImperial = convertedFeet === null
    ? null
    : formatFeetAndInches(convertedFeet, { inchDenominator });

  const widthValue = parseFiniteNumber(widthInput);
  const depthValue = parseFiniteNumber(depthInput);
  const areaSqFt = widthValue === null || depthValue === null
    ? null
    : getAreaSqFt(widthValue, depthValue);

  return (
    <aside className="units-inspector" role="dialog" aria-label="Units inspector" aria-modal="false">
      <header className="units-inspector-header">
        <div>
          <strong>Units Inspector</strong>
          <span>Manual parse, format, conversion, and area checks</span>
        </div>

        <button type="button" className="units-inspector-close" onClick={onClose}>
          Close
        </button>
      </header>

      <section className="units-inspector-section">
        <div className="units-inspector-title-row">
          <h3>Imperial Parse</h3>
          <span>US ft-in input</span>
        </div>

        <label className="units-inspector-field">
          <span>Input</span>
          <input
            type="text"
            value={parseInput}
            onChange={(event) => setParseInput(event.target.value)}
          />
        </label>

        <label className="units-inspector-field">
          <span>Bare number unit</span>
          <select
            value={defaultUnit}
            onChange={(event) => setDefaultUnit(event.target.value as DefaultLengthUnit)}
          >
            <option value="ft">Feet</option>
            <option value="in">Inches</option>
          </select>
        </label>

        <div className="units-sample-row" aria-label="Sample unit inputs">
          {sampleInputs.map((sample) => (
            <button
              key={sample}
              type="button"
              className={`units-sample-chip ${parseInput === sample ? "is-active" : ""}`}
              onClick={() => setParseInput(sample)}
            >
              {sample}
            </button>
          ))}
        </div>

        <p className="units-inspector-note">
          Bare single numbers use the selected default unit.
          {" "}
          <code>''</code>
          {" "}
          is treated as inches, and explicit
          {" "}
          <code>'</code>
          {" "}
          or
          {" "}
          <code>"</code>
          {" "}
          markers still win over the default. Markerless shorthand like
          {" "}
          <code>12 3 3/4</code>
          {" "}
          is read as feet, inches, fraction.
        </p>

        <dl className="units-result-list">
          <div>
            <dt>Status</dt>
            <dd>{parsedFeet === null ? "Invalid input" : "Parsed"}</dd>
          </div>
          <div>
            <dt>Decimal feet</dt>
            <dd>{parsedFeet === null ? "Invalid input" : formatNumber(parsedFeet)}</dd>
          </div>
          <div>
            <dt>Meters</dt>
            <dd>{parsedMeters === null ? "Invalid input" : formatNumber(parsedMeters)}</dd>
          </div>
          <div>
            <dt>Normalized</dt>
            <dd>{parsedNormalized ?? "Invalid input"}</dd>
          </div>
        </dl>
      </section>

      <section className="units-inspector-section">
        <div className="units-inspector-title-row">
          <h3>Feet Format</h3>
          <span>Decimal feet to UI string</span>
        </div>

        <label className="units-inspector-field">
          <span>Feet</span>
          <input
            type="text"
            inputMode="decimal"
            value={feetInput}
            onChange={(event) => setFeetInput(event.target.value)}
          />
        </label>

        <label className="units-inspector-field">
          <span>Denominator</span>
          <select
            value={String(inchDenominator)}
            onChange={(event) => setInchDenominator(Number(event.target.value) as InchDenominator)}
          >
            {denominatorOptions.map((option) => (
              <option key={option} value={option}>
                1/{option}"
              </option>
            ))}
          </select>
        </label>

        <dl className="units-result-list">
          <div>
            <dt>Formatted</dt>
            <dd>{formattedFeet ?? "Invalid number"}</dd>
          </div>
          <div>
            <dt>Meters</dt>
            <dd>{formattedFeetMeters === null ? "Invalid number" : formatNumber(formattedFeetMeters)}</dd>
          </div>
        </dl>
      </section>

      <section className="units-inspector-section">
        <div className="units-inspector-title-row">
          <h3>Meters Convert</h3>
          <span>Metric to imperial display</span>
        </div>

        <label className="units-inspector-field">
          <span>Meters</span>
          <input
            type="text"
            inputMode="decimal"
            value={metersInput}
            onChange={(event) => setMetersInput(event.target.value)}
          />
        </label>

        <dl className="units-result-list">
          <div>
            <dt>Feet</dt>
            <dd>{convertedFeet === null ? "Invalid number" : formatNumber(convertedFeet)}</dd>
          </div>
          <div>
            <dt>Formatted</dt>
            <dd>{convertedImperial ?? "Invalid number"}</dd>
          </div>
        </dl>
      </section>

      <section className="units-inspector-section">
        <div className="units-inspector-title-row">
          <h3>Area</h3>
          <span>Rectangular sq ft</span>
        </div>

        <div className="units-inspector-grid">
          <label className="units-inspector-field">
            <span>Width ft</span>
            <input
              type="text"
              inputMode="decimal"
              value={widthInput}
              onChange={(event) => setWidthInput(event.target.value)}
            />
          </label>

          <label className="units-inspector-field">
            <span>Depth ft</span>
            <input
              type="text"
              inputMode="decimal"
              value={depthInput}
              onChange={(event) => setDepthInput(event.target.value)}
            />
          </label>
        </div>

        <dl className="units-result-list">
          <div>
            <dt>Area sq ft</dt>
            <dd>{areaSqFt === null ? "Invalid number" : formatNumber(areaSqFt, 2)}</dd>
          </div>
        </dl>
      </section>
    </aside>
  );
}
