import { useSyncExternalStore } from "react";
import { createClient, type Session, type SupabaseClient, type User } from "@supabase/supabase-js";

export type AuthStatus = "loading" | "signed_out" | "signed_in";

export type AuthSnapshot = {
  status: AuthStatus;
  session: Session | null;
  user: User | null;
  error: string | null;
  pendingEmail: string;
};

const listeners = new Set<() => void>();

let authSnapshot: AuthSnapshot = {
  status: "loading",
  session: null,
  user: null,
  error: null,
  pendingEmail: ""
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
    pendingEmail: ""
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
  return error?.message?.trim() || fallback;
}

function getBrowserRedirectUrl(): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }

  return new URL("/editor", window.location.origin).toString();
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
    clientError = "Missing VITE_SUPABASE_URL or VITE_SUPABASE_PUBLISHABLE_KEY.";
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

function applySession(session: Session | null, error: string | null = null) {
  setSnapshot((current) => ({
    ...current,
    status: session ? "signed_in" : "signed_out",
    session,
    user: session?.user ?? null,
    error,
    pendingEmail: session ? "" : current.pendingEmail
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
        user: null
      }));
      return;
    }

    if (!authSubscription) {
      const { data } = clientResult.client.auth.onAuthStateChange((_event, session) => {
        applySession(session);
      });
      authSubscription = data.subscription;
    }

    const { data, error } = await clientResult.client.auth.getSession();

    if (error) {
      setSnapshot((current) => ({
        ...current,
        status: "signed_out",
        error: getErrorMessage(error, "Unable to restore the current session."),
        session: null,
        user: null
      }));
      return;
    }

    applySession(data.session);
  })();

  return bootstrapPromise;
}

export async function sendLoginEmail(email: string): Promise<{ error: string | null }> {
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

  const redirectTo = getBrowserRedirectUrl();
  const { error } = await clientResult.client.auth.signInWithOtp({
    email,
    options: redirectTo ? { emailRedirectTo: redirectTo } : undefined
  });

  if (error) {
    const message = getErrorMessage(error, "Unable to send the sign-in email.");
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: message
    }));
    return { error: message };
  }

  setSnapshot((current) => ({
    ...current,
    status: current.session ? "signed_in" : "signed_out",
    error: null,
    pendingEmail: email
  }));

  return { error: null };
}

export async function verifyEmailOtp(email: string, token: string): Promise<{ error: string | null }> {
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

  const { data, error } = await clientResult.client.auth.verifyOtp({
    email,
    token,
    type: "email"
  });

  if (error) {
    const message = getErrorMessage(error, "Unable to verify the email code.");
    setSnapshot((current) => ({
      ...current,
      status: "signed_out",
      error: message
    }));
    return { error: message };
  }

  if (data.session) {
    applySession(data.session);
  } else {
    setSnapshot((current) => ({
      ...current,
      error: null
    }));
  }

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
    const message = getErrorMessage(error, "Unable to log out.");
    setSnapshot((current) => ({
      ...current,
      error: message
    }));
    return { error: message };
  }

  applySession(null);
  return { error: null };
}
