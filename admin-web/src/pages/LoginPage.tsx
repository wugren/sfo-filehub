import { useEffect, useSyncExternalStore, useState, type FormEvent, type ReactElement } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { sessionStore } from "../api/session";
import { ApiError } from "../api/errors";
import { Icon } from "../components/icons";
import { Btn, ErrorBanner, Inp, LanguageSwitcher } from "../components/ui";
import { useT } from "../i18n";

export function LoginPage(): ReactElement {
  const [userName, setUserName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const t = useT();

  const version = useSyncExternalStore(sessionStore.subscribe, sessionStore.getSnapshot);
  const authenticated = sessionStore.state === "authenticated";

  useEffect(() => {
    if (!authenticated) {
      return;
    }
    const next = params.get("next");
    navigate(next && next.startsWith("/") ? next : "/projects", { replace: true });
  }, [authenticated, navigate, params]);

  void version;

  async function onSubmit(event: FormEvent): Promise<void> {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      await sessionStore.login(userName.trim(), password);
      const next = params.get("next");
      navigate(next && next.startsWith("/") ? next : "/projects", { replace: true });
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : "login failed";
      setError(message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="login-page">
      <div className="login-lang">
        <LanguageSwitcher compact />
      </div>
      <div className="login-inner">
        <div className="login-brand">
          <Icon name="terminal" size={18} />
          <span className="login-brand-name">filehub</span>
        </div>
        <p className="login-subtitle">{t("login.subtitle")}</p>
        <form onSubmit={(event) => { void onSubmit(event); }} className="login-card">
          <label className="field">
            <span className="field-label">{t("login.username")}</span>
            <Inp
              name="user_name"
              autoComplete="username"
              placeholder="admin"
              value={userName}
              onChange={(event) => setUserName(event.target.value)}
              required
              autoFocus
            />
          </label>
          <label className="field">
            <span className="field-label">{t("login.password")}</span>
            <Inp
              name="password"
              type="password"
              autoComplete="current-password"
              placeholder="••••••••"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </label>
          <ErrorBanner message={error} onDismiss={() => setError("")} />
          <Btn type="submit" variant="primary" disabled={busy} className="login-submit">
            {busy ? t("login.signingIn") : t("login.signIn")}
          </Btn>
        </form>
        <p className="login-hint">{t("login.storageHint")}</p>
      </div>
    </div>
  );
}

export function statusMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.message;
  }
  return error instanceof Error ? error.message : "操作失败，请重试";
}
