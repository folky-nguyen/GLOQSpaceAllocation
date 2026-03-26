import { useSyncExternalStore } from "react";
import { createClient, type Session, type SupabaseClient, type User } from "@supabase/supabase-js";

export type AuthStatus = "loading" | "signed_out" | "signed_in";
export type PendingOtpType = "email" | "recovery";

export type AuthSnapshot = {
  status: AuthStatus;
  session: Session | null;
  user: User | null;
  error: string | null;
  pendingEmail: string;
  pendingOtpType: PendingOtpType | null;
  recoveryReady: boolean;
};

const listeners = new Set<() => void>();

let authSnapshot: AuthSnapshot = {
  status: "loading",
  session: null,
  user: null,
  error: null,
  pendingEmail: "",
  pendingOtpType: null,
  recoveryReady: false
};

let client: SupabaseClient | null = null;
let clientError: string | null = null;
let bootstrapPromise: Promise<void> | null = null;
let authSubscription: { unsubscribe: () => void } | null = null;

function isLocalDevAuthBypassed(): boolean {
  if (!import.meta.env.DEV) {
    return false;
  }

  if (import.meta.env.VITE_LOCAL_AUTH_BYPASS?.trim().toLowerCase() !== "true") {
    return false;
  }

  if (typeof window === "undefined") {
    return true;
  }

  return window.location.hostname === "localhost" || window.location.hostname === "127.0.0.1";
}

function getLocalDevUser(): User {
  return {
    id: "local-dev-user",
    app_metadata: { provider: "email" },
    user_metadata: { name: "Local Dev" },
    aud: "authenticated",
    created_at: "1970-01-01T00:00:00.000Z",
    email: "local-dev@gloq.local"
  } as User;
}

function applyLocalDevBypass() {
  setSnapshot({
    status: "signed_in",
    session: null,
    user: getLocalDevUser(),
    error: null,
    pendingEmail: "",
    pendingOtpType: null,
    recoveryReady: false
  });
}

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

function setSnapshot(updater: AuthSnapshot | ((current: AuthSnapshot) => AuthSnapshot)) {
  authSnapshot = typeof updater === "function" ? updater(authSnapshot) : updater;
  emitChange();
}

function getErrorMessage(error: { message?: string } | null | undefined, fallback: string): string {
  const detail = error?.message?.trim();

  if (!detail || detail === fallback) {
    return fallback;
  }

  return `${fallback} Detail: ${detail}`;
}

function getBrowserRedirectUrl(path: "/login" | "/editor" = "/login"): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }

  return new URL(path, window.location.origin).toString();
}

function getSupabaseClient(): { client: SupabaseClient | null; error: string | null } {
  if (client) {
    return { client, error: null };
  }

  if (clientError) {
    return { client: null, error: clientError };
  }

  const url = import.meta.env.VITE_SUPABASE_URL?.trim();
  const publishableKey = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY?.trim();

  if (!url || !publishableKey) {
    clientError = "Supabase browser auth is not configured. Add VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY.";
    return { client: null, error: clientError };
  }

  client = createClient(url, publishableKey, {
    auth: {
      autoRefreshToken: true,
      detectSessionInUrl: true,
      persistSession: true
    }
  });

  return { client, error: null };
}

function applySession(
  session: Session | null,
  error: string | null = null,
  options?: { recoveryReady?: boolean }
) {
  setSnapshot({
    status: session ? "signed_in" : "signed_out",
    session,
    user: session?.user ?? null,
    error,
    pendingEmail: "",
    pendingOtpType: null,
    recoveryReady: session ? Boolean(options?.recoveryReady) : false
  });
}

function beginPublicAuthAction() {
  setSnapshot((current) => ({
    ...current,
    error: null,
    pendingEmail: "",
    pendingOtpType: null,
    recoveryReady: false
  }));
}

function setPendingOtp(email: string, type: PendingOtpType) {
  setSnapshot((current) => ({
    ...current,
    status: "signed_out",
    session: null,
    user: null,
    error: null,
    pendingEmail: email,
    pendingOtpType: type,
    recoveryReady: false
  }));
}

export function subscribeAuth(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useAuth(): AuthSnapshot {
  return useSyncExternalStore(subscribeAuth, () => authSnapshot, () => authSnapshot);
}

export async function bootstrapAuth(): Promise<void> {
  if (bootstrapPromise) {
    return bootstrapPromise;
  }

  bootstrapPromise = (async () => {
    if (isLocalDevAuthBypassed()) {
      applyLocalDevBypass();
      return;
    }

    const clientResult = getSupabaseClient();

    if (!clientResult.client) {
      setSnapshot((current) => ({
        ...current,
        status: "signed_out",
        error: clientResult.error,
        session: null,
        user: null,
        recoveryReady: false
      }));
      return;
    }

    if (!authSubscription) {
      const { data } = clientResult.client.auth.onAuthStateChange((event, session) => {
        const recoveryReady = event === "PASSWORD_RECOVERY"
          || (session !== null && authSnapshot.pendingOtpType === "recovery")
          || (session !== null && authSnapshot.recoveryReady);

        applySession(session, null, { recoveryReady });
      });
      authSubscription = data.subscription;
    }

    const { data, error } = await clientResult.client.auth.getSession();

    if (error) {
      setSnapshot((current) => ({
        ...current,
        status: "signed_out",
        error: getErrorMessage(error, "Could not restore the current session."),
        session: null,
        user: null,
        recoveryReady: false
      }));
      return;
    }

    applySession(data.session);
  })();

  return bootstrapPromise;
}

export async function signInWithPassword(email: string, password: string): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();
  beginPublicAuthAction();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const { data, error } = await clientResult.client.auth.signInWithPassword({ email, password });

  if (error || !data.session) {
    const message = getErrorMessage(error, "Could not sign in with email and password.");
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      session: null,
      user: null,
      error: message,
      recoveryReady: false
    }));
    return { error: message };
  }

  applySession(data.session);
  return { error: null };
}

export async function signUpWithPassword(email: string, password: string): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();
  beginPublicAuthAction();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const redirectTo = getBrowserRedirectUrl("/login");
  const { data, error } = await clientResult.client.auth.signUp({
    email,
    password,
    options: redirectTo ? { emailRedirectTo: redirectTo } : undefined
  });

  if (error) {
    const message = getErrorMessage(error, "Could not create the account.");
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: message
    }));
    return { error: message };
  }

  if (data.session) {
    applySession(data.session);
    return { error: null };
  }

  setPendingOtp(email, "email");
  return { error: null };
}

export async function sendRecoveryEmail(email: string): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();
  beginPublicAuthAction();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const redirectTo = getBrowserRedirectUrl("/login");
  const { error } = await clientResult.client.auth.resetPasswordForEmail(
    email,
    redirectTo ? { redirectTo } : undefined
  );

  if (error) {
    const message = getErrorMessage(error, "Could not send the recovery email.");
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: message
    }));
    return { error: message };
  }

  setPendingOtp(email, "recovery");
  return { error: null };
}

export async function verifyEmailOtp(
  email: string,
  token: string,
  type: PendingOtpType
): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const { data, error } = await clientResult.client.auth.verifyOtp({ email, token, type });

  if (error) {
    const fallback = type === "recovery"
      ? "Could not verify the recovery code."
      : "Could not verify the email code.";
    const message = getErrorMessage(error, fallback);
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: message
    }));
    return { error: message };
  }

  if (type === "recovery") {
    if (!data.session) {
      const message = "Could not open the password reset session.";
      setSnapshot((current) => ({
        ...current,
        status: "signed_out",
        error: message,
        recoveryReady: false
      }));
      return { error: message };
    }

    applySession(data.session, null, { recoveryReady: true });
    return { error: null };
  }

  if (data.session) {
    applySession(data.session);
  } else {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: null,
      pendingEmail: "",
      pendingOtpType: null,
      recoveryReady: false
    }));
  }

  return { error: null };
}

export async function updatePassword(password: string): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const { error } = await clientResult.client.auth.updateUser({ password });

  if (error) {
    const message = getErrorMessage(error, "Could not update the password.");
    setSnapshot((current) => ({
      ...current,
      error: message
    }));
    return { error: message };
  }

  const signOutResult = await clientResult.client.auth.signOut();

  if (signOutResult.error) {
    const message = getErrorMessage(signOutResult.error, "Password updated, but could not sign out.");
    setSnapshot((current) => ({
      ...current,
      error: message
    }));
    return { error: message };
  }

  applySession(null);
  return { error: null };
}

export async function logout(): Promise<{ error: string | null }> {
  if (isLocalDevAuthBypassed()) {
    applyLocalDevBypass();
    return { error: null };
  }

  await bootstrapAuth();

  const clientResult = getSupabaseClient();

  if (!clientResult.client) {
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: clientResult.error
    }));
    return { error: clientResult.error };
  }

  const { error } = await clientResult.client.auth.signOut();

  if (error) {
    const message = getErrorMessage(error, "Could not sign out.");
    setSnapshot((current) => ({
      ...current,
      error: message
    }));
    return { error: message };
  }

  applySession(null);
  return { error: null };
}
