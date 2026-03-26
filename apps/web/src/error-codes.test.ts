import { describe, expect, it } from "vitest";
import { formatTrappedErrorMessage, getTrappedErrorCode } from "./error-codes";

describe("error code formatting", () => {
  it("looks up a trapped error code from the canonical summary", () => {
    expect(getTrappedErrorCode("The browser exposed WebGPU, but this device did not return a usable graphics adapter."))
      .toBe("WEB-3D-002");
  });

  it("injects the error code before detail text on single-string surfaces", () => {
    expect(formatTrappedErrorMessage("Could not sign in with email and password. Detail: Invalid login credentials"))
      .toBe("Could not sign in with email and password. Code: WEB-AUTH-003. Detail: Invalid login credentials");
  });

  it("appends the error code once and avoids duplicate markers", () => {
    expect(formatTrappedErrorMessage("Email is required."))
      .toBe("Email is required. Code: WEB-LOGIN-001.");
    expect(formatTrappedErrorMessage("Email is required. Code: WEB-LOGIN-001."))
      .toBe("Email is required. Code: WEB-LOGIN-001.");
  });

  it("leaves unknown messages unchanged", () => {
    expect(formatTrappedErrorMessage("Unexpected browser state."))
      .toBe("Unexpected browser state.");
  });
});
