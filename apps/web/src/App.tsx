import { useEffect, useState, type FormEvent } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { bootstrapAuth, sendLoginEmail, useAuth, verifyEmailOtp } from "./auth";
import EditorShell from "./editor-shell";

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

  if (auth.status === "signed_out") {
    return <Navigate to="/login" replace />;
  }

  return <EditorShell />;
}

function HomeRedirect() {
  const auth = useAuth();

  if (auth.status === "loading") {
    return <LoadingScreen />;
  }

  return <Navigate replace to={auth.status === "signed_in" ? "/editor" : "/login"} />;
}

function LoginPage() {
  const auth = useAuth();
  const [email, setEmail] = useState(auth.pendingEmail);
  const [otp, setOtp] = useState("");
  const [action, setAction] = useState<"send" | "verify" | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  if (auth.status === "loading") {
    return <LoadingScreen />;
  }

  if (auth.status === "signed_in") {
    return <Navigate to="/editor" replace />;
  }

  const handleSend = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedEmail = email.trim();

    if (!trimmedEmail) {
      setLocalError("Email is required.");
      return;
    }

    setAction("send");
    setLocalError(null);

    const result = await sendLoginEmail(trimmedEmail);

    if (!result.error) {
      setOtp("");
    } else {
      setLocalError(result.error);
    }

    setAction(null);
  };

  const handleVerify = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const verificationEmail = auth.pendingEmail || email.trim();
    const trimmedOtp = otp.trim();

    if (!verificationEmail) {
      setLocalError("Send the sign-in email first.");
      return;
    }

    if (!trimmedOtp) {
      setLocalError("OTP code is required.");
      return;
    }

    setAction("verify");
    setLocalError(null);

    const result = await verifyEmailOtp(verificationEmail, trimmedOtp);

    if (result.error) {
      setLocalError(result.error);
    }

    setAction(null);
  };

  const displayError = localError ?? auth.error;

  return (
    <main className="auth-page">
      <section className="auth-card" aria-labelledby="login-title">
        <div className="auth-copy">
          <span className="auth-kicker">Supabase Auth</span>
          <h1 id="login-title">Open the editor</h1>
          <p>Use email passwordless auth. Supabase can send either a magic link or a one-time code.</p>
        </div>

        <form className="auth-form" onSubmit={(event) => void handleSend(event)}>
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
              {action === "send" ? "Sending..." : "Send link or code"}
            </button>
          </div>
        </form>

        {auth.pendingEmail ? (
          <div className="auth-note">
            <strong>Email sent to {auth.pendingEmail}.</strong>
            <span>Click the magic link, or paste the OTP code below if your template sends a token.</span>
          </div>
        ) : null}

        <form className="auth-form auth-form-secondary" onSubmit={(event) => void handleVerify(event)}>
          <label className="auth-field">
            <span>OTP code</span>
            <input
              autoComplete="one-time-code"
              inputMode="numeric"
              name="otp"
              type="text"
              value={otp}
              onChange={(event) => setOtp(event.target.value)}
            />
          </label>

          <div className="auth-actions">
            <button
              type="submit"
              className="auth-secondary"
              disabled={action !== null || !auth.pendingEmail}
            >
              {action === "verify" ? "Verifying..." : "Verify code"}
            </button>
          </div>
        </form>

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
