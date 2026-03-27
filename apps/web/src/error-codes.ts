const DETAIL_MARKER = " Detail:";
const CODE_PATTERN = /\bCode:\s+[A-Z]+-[A-Z]+-\d+\./;

const trappedErrorCodeBySummary = new Map<string, string>([
  ["Email is required.", "WEB-LOGIN-001"],
  ["Password is required.", "WEB-LOGIN-002"],
  ["OTP code is required.", "WEB-LOGIN-003"],
  ["New password is required.", "WEB-LOGIN-004"],
  ["Supabase browser auth is not configured. Add VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY.", "WEB-AUTH-001"],
  ["Could not restore the current session.", "WEB-AUTH-002"],
  ["Could not sign in with email and password.", "WEB-AUTH-003"],
  ["Could not create the account.", "WEB-AUTH-004"],
  ["Could not send the recovery email.", "WEB-AUTH-005"],
  ["Could not verify the recovery code.", "WEB-AUTH-006"],
  ["Could not verify the email code.", "WEB-AUTH-007"],
  ["Could not open the password reset session.", "WEB-AUTH-008"],
  ["Could not update the password.", "WEB-AUTH-009"],
  ["Password updated, but could not sign out.", "WEB-AUTH-010"],
  ["Could not sign out.", "WEB-AUTH-011"],
  ["Stories below grade must be a whole number greater than or equal to 0.", "WEB-LEVEL-001"],
  ["Stories on grade must be a whole number greater than or equal to 0.", "WEB-LEVEL-002"],
  ["Auto-generate requires at least one story.", "WEB-LEVEL-003"],
  ["Story height must be a positive feet-inch value.", "WEB-LEVEL-004"],
  ["Site boundary must include at least 3 valid points.", "WEB-SITE-001"],
  ["Site boundary must enclose a valid area.", "WEB-SITE-002"],
  ["Site boundary cannot contain a zero-length edge.", "WEB-SITE-003"],
  ["Setbacks must resolve to a valid building footprint.", "WEB-SITE-004"],
  ["Setbacks collapse the building footprint.", "WEB-SITE-005"],
  ["Setbacks exceed the available site depth.", "WEB-SITE-006"],
  ["Setback must be greater than or equal to 0.", "WEB-SITE-007"],
  ["This browser does not expose `navigator.gpu`, so the wasm renderer cannot start.", "WEB-3D-001"],
  ["The browser exposed WebGPU, but this device did not return a usable graphics adapter.", "WEB-3D-002"],
  ["The web app and the checked-in wasm renderer package are out of sync.", "WEB-3D-003"],
  ["The wasm renderer threw before the first frame could be drawn.", "WEB-3D-004"],
  ["The renderer started, but failed while sending the current scene to WebGPU.", "WEB-3D-005"]
]);

function getMessageSummary(message: string): string {
  const trimmed = message.trim();
  const detailIndex = trimmed.indexOf(DETAIL_MARKER);
  const summary = detailIndex >= 0 ? trimmed.slice(0, detailIndex) : trimmed;
  return summary.replace(CODE_PATTERN, "").trim();
}

export function getTrappedErrorCode(message: string | null | undefined): string | null {
  if (!message) {
    return null;
  }

  const summary = getMessageSummary(message);
  return trappedErrorCodeBySummary.get(summary) ?? null;
}

export function formatTrappedErrorMessage(message: string | null | undefined): string | null {
  if (!message) {
    return null;
  }

  const trimmed = message.trim();

  if (!trimmed || CODE_PATTERN.test(trimmed)) {
    return trimmed || null;
  }

  const code = getTrappedErrorCode(trimmed);

  if (!code) {
    return trimmed;
  }

  const detailIndex = trimmed.indexOf(DETAIL_MARKER);

  if (detailIndex < 0) {
    return `${trimmed} Code: ${code}.`;
  }

  const summary = trimmed.slice(0, detailIndex).trim();
  const detail = trimmed.slice(detailIndex + 1).trim();
  return `${summary} Code: ${code}. ${detail}`;
}
