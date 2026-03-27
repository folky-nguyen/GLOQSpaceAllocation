import { useEffect, useState, type FormEvent } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import {
  bootstrapAuth,
  sendRecoveryEmail,
  signInWithPassword,
  signUpWithPassword,
  updatePassword,
  useAuth,
  verifyEmailOtp
} from "./auth";
import { formatTrappedErrorMessage } from "./error-codes";
import EditorShell from "./editor-shell";

type AuthMode = "login" | "signup" | "recovery";
type AuthAction =
  | "login"
  | "signup"
  | "send-recovery"
  | "verify-signup"
  | "verify-recovery"
  | "update-password"
  | null;

function LoadingScreen() {
  return (
    <main className="auth-loading">
      <div className="auth-loading-card">
        <strong>Loading session</strong>
        <span>Checking Supabase auth state.</span>
      </div>
    </main>
  );
}

function ProtectedEditorRoute() {
  const auth = useAuth();

  if (auth.status === "loading") {
    return <LoadingScreen />;
  }

  if (auth.status === "signed_out" || auth.recoveryReady) {
    return <Navigate to="/login" replace />;
  }

  return <EditorShell />;
}

function HomeRedirect() {
  const auth = useAuth();

  if (auth.status === "loading") {
    return <LoadingScreen />;
  }

  return <Navigate replace to={auth.status === "signed_in" && !auth.recoveryReady ? "/editor" : "/login"} />;
}

function getInitialMode(auth: ReturnType<typeof useAuth>): AuthMode {
  if (auth.recoveryReady || auth.pendingOtpType === "recovery") {
    return "recovery";
  }

  if (auth.pendingOtpType === "email") {
    return "signup";
  }

  return "login";
}

function LoginPage() {
  const auth = useAuth();
  const [mode, setMode] = useState<AuthMode>(() => getInitialMode(auth));
  const [email, setEmail] = useState(auth.pendingEmail);
  const [password, setPassword] = useState("");
  const [otp, setOtp] = useState("");
  const [nextPassword, setNextPassword] = useState("");
  const [action, setAction] = useState<AuthAction>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (auth.pendingEmail && !email) {
      setEmail(auth.pendingEmail);
    }
  }, [auth.pendingEmail, email]);

  useEffect(() => {
    if (auth.recoveryReady || auth.pendingOtpType === "recovery") {
      setMode("recovery");
      return;
    }

    if (auth.pendingOtpType === "email") {
      setMode("signup");
    }
  }, [auth.pendingOtpType, auth.recoveryReady]);

  if (auth.status === "loading") {
    return <LoadingScreen />;
  }

  if (auth.status === "signed_in" && !auth.recoveryReady) {
    return <Navigate to="/editor" replace />;
  }

  const switchMode = (nextMode: AuthMode) => {
    if (auth.recoveryReady && nextMode !== "recovery") {
      return;
    }

    setMode(nextMode);
    setLocalError(null);
    setNotice(null);
    setOtp("");
    setNextPassword("");

    if (nextMode !== "signup") {
      setPassword("");
    }
  };

  const handleLogin = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    if (!password) {
      setLocalError("Password is required.");
      return;
    }

    setAction("login");
    setLocalError(null);
    setNotice(null);

    const result = await signInWithPassword(trimmedEmail, password);

    if (result.error) {
      setLocalError(result.error);
    }

    setAction(null);
  };

  const handleSignUp = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    if (!password) {
      setLocalError("Password is required.");
      return;
    }

    setAction("signup");
    setLocalError(null);
    setNotice(null);

    const result = await signUpWithPassword(trimmedEmail, password);

    if (result.error) {
      setLocalError(result.error);
    } else {
      setOtp("");
      setNotice("Account created. If email confirmation is enabled, enter the OTP code below.");
    }

    setAction(null);
  };

  const handleVerifySignup = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();
    const trimmedOtp = otp.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    if (!trimmedOtp) {
      setLocalError("OTP code is required.");
      return;
    }

    setAction("verify-signup");
    setLocalError(null);
    setNotice(null);

    const result = await verifyEmailOtp(trimmedEmail, trimmedOtp, "email");

    if (result.error) {
      setLocalError(result.error);
    }

    setAction(null);
  };

  const handleSendRecovery = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    setAction("send-recovery");
    setLocalError(null);
    setNotice(null);

    const result = await sendRecoveryEmail(trimmedEmail);

    if (result.error) {
      setLocalError(result.error);
    } else {
      setOtp("");
      setNextPassword("");
      setNotice("Recovery email sent. Enter the OTP code below, or open the recovery link to continue here.");
    }

    setAction(null);
  };

  const handleVerifyRecovery = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();
    const trimmedOtp = otp.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    if (!trimmedOtp) {
      setLocalError("OTP code is required.");
      return;
    }

    setAction("verify-recovery");
    setLocalError(null);
    setNotice(null);

    const result = await verifyEmailOtp(trimmedEmail, trimmedOtp, "recovery");

    if (result.error) {
      setLocalError(result.error);
    } else {
      setNotice("Recovery verified. Set a new password below.");
      setNextPassword("");
    }

    setAction(null);
  };

  const handleUpdatePassword = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!nextPassword) {
      setLocalError("New password is required.");
      return;
    }

    setAction("update-password");
    setLocalError(null);
    setNotice(null);

    const result = await updatePassword(nextPassword);

    if (result.error) {
      setLocalError(result.error);
    } else {
      setMode("login");
      setPassword("");
      setOtp("");
      setNextPassword("");
      setNotice("Password updated. Log in with your new password.");
    }

    setAction(null);
  };

  const displayError = formatTrappedErrorMessage(localError ?? auth.error);

  const copyByMode: Record<AuthMode, { title: string; body: string }> = {
    login: {
      title: "Open the editor",
      body: "Sign in with your email and password."
    },
    signup: {
      title: "Create your account",
      body: "Choose an email and password. If your project requires confirmation, verify the OTP code below."
    },
    recovery: {
      title: "Reset your password",
      body: auth.recoveryReady
        ? "Set a new password for your account."
        : "Send a recovery email, then verify the OTP code to continue."
    }
  };

  return (
    <main className="auth-page">
      <section className="auth-card" aria-labelledby="login-title">
        <div className="auth-copy">
          <span className="auth-kicker">Supabase Auth</span>
          <h1 id="login-title">{copyByMode[mode].title}</h1>
          <p>{copyByMode[mode].body}</p>
        </div>

        <div className="auth-mode-row" role="tablist" aria-label="Authentication modes">
          <button
            type="button"
            className={`auth-mode-button ${mode === "login" ? "is-active" : ""}`}
            aria-pressed={mode === "login"}
            disabled={action !== null || auth.recoveryReady}
            onClick={() => switchMode("login")}
          >
            Log in
          </button>
          <button
            type="button"
            className={`auth-mode-button ${mode === "signup" ? "is-active" : ""}`}
            aria-pressed={mode === "signup"}
            disabled={action !== null || auth.recoveryReady}
            onClick={() => switchMode("signup")}
          >
            Create account
          </button>
          <button
            type="button"
            className={`auth-mode-button ${mode === "recovery" ? "is-active" : ""}`}
            aria-pressed={mode === "recovery"}
            disabled={action !== null}
            onClick={() => switchMode("recovery")}
          >
            Forgot password
          </button>
        </div>

        {mode === "login" ? (
          <form className="auth-form" onSubmit={(event) => void handleLogin(event)}>
            <label className="auth-field">
              <span>Email</span>
              <input
                autoComplete="email"
                inputMode="email"
                name="email"
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </label>

            <label className="auth-field">
              <span>Password</span>
              <input
                autoComplete="current-password"
                name="password"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </label>

            <div className="auth-actions">
              <button type="submit" className="auth-primary" disabled={action !== null}>
                {action === "login" ? "Signing in..." : "Log in"}
              </button>
            </div>
          </form>
        ) : null}

        {mode === "signup" ? (
          <>
            <form className="auth-form" onSubmit={(event) => void handleSignUp(event)}>
              <label className="auth-field">
                <span>Email</span>
                <input
                  autoComplete="email"
                  inputMode="email"
                  name="email"
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                />
              </label>

              <label className="auth-field">
                <span>Password</span>
                <input
                  autoComplete="new-password"
                  name="password"
                  type="password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                />
              </label>

              <div className="auth-actions">
                <button type="submit" className="auth-primary" disabled={action !== null}>
                  {action === "signup" ? "Creating..." : "Create account"}
                </button>
              </div>
            </form>

            <div className="auth-note">
              <strong>Signup OTP</strong>
              <span>After creating the account, enter the code here if your project requires email confirmation.</span>
            </div>

            <form className="auth-form auth-form-secondary" onSubmit={(event) => void handleVerifySignup(event)}>
              <label className="auth-field">
                <span>OTP code</span>
                <input
                  autoComplete="one-time-code"
                  inputMode="numeric"
                  name="signup-otp"
                  type="text"
                  value={otp}
                  onChange={(event) => setOtp(event.target.value)}
                />
              </label>

              <div className="auth-actions">
                <button type="submit" className="auth-secondary" disabled={action !== null}>
                  {action === "verify-signup" ? "Verifying..." : "Verify code"}
                </button>
              </div>
            </form>
          </>
        ) : null}

        {mode === "recovery" && !auth.recoveryReady ? (
          <>
            <form className="auth-form" onSubmit={(event) => void handleSendRecovery(event)}>
              <label className="auth-field">
                <span>Email</span>
                <input
                  autoComplete="email"
                  inputMode="email"
                  name="email"
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                />
              </label>

              <div className="auth-actions">
                <button type="submit" className="auth-primary" disabled={action !== null}>
                  {action === "send-recovery" ? "Sending..." : "Send recovery code"}
                </button>
              </div>
            </form>

            <div className="auth-note">
              <strong>Recovery OTP</strong>
              <span>Enter the code from the recovery email here. Opening the recovery link also returns to this screen.</span>
            </div>

            <form className="auth-form auth-form-secondary" onSubmit={(event) => void handleVerifyRecovery(event)}>
              <label className="auth-field">
                <span>OTP code</span>
                <input
                  autoComplete="one-time-code"
                  inputMode="numeric"
                  name="recovery-otp"
                  type="text"
                  value={otp}
                  onChange={(event) => setOtp(event.target.value)}
                />
              </label>

              <div className="auth-actions">
                <button type="submit" className="auth-secondary" disabled={action !== null}>
                  {action === "verify-recovery" ? "Verifying..." : "Verify code"}
                </button>
              </div>
            </form>
          </>
        ) : null}

        {mode === "recovery" && auth.recoveryReady ? (
          <form className="auth-form" onSubmit={(event) => void handleUpdatePassword(event)}>
            <label className="auth-field">
              <span>New password</span>
              <input
                autoComplete="new-password"
                name="new-password"
                type="password"
                value={nextPassword}
                onChange={(event) => setNextPassword(event.target.value)}
              />
            </label>

            <div className="auth-actions">
              <button type="submit" className="auth-primary" disabled={action !== null}>
                {action === "update-password" ? "Saving..." : "Set new password"}
              </button>
            </div>
          </form>
        ) : null}

        {notice ? (
          <div className="auth-note">
            <strong>Notice</strong>
            <span>{notice}</span>
          </div>
        ) : null}

        {displayError ? <p className="auth-error">{displayError}</p> : null}
      </section>
    </main>
  );
}

function AppRoutes() {
  useEffect(() => {
    void bootstrapAuth();
  }, []);

  return (
    <Routes>
      <Route path="/" element={<HomeRedirect />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/editor" element={<ProtectedEditorRoute />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}
