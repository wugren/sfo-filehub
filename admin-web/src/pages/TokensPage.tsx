import { useCallback, useEffect, useState, type ReactElement } from "react";
import { ApiClient } from "../api/client";
import {
  ALL_SCOPES,
  encodeProjectScope,
  type Project,
  type ProjectScopeDto,
  type Scope,
  type TokenSummary,
  type TokenUpdateInput,
} from "../api/contract";
import { sessionStore, withAuthRetry } from "../api/session";
import { Icon } from "../components/icons";
import {
  Badge,
  Btn,
  Confirm,
  ErrorBanner,
  Field,
  Inp,
  Modal,
} from "../components/ui";
import { formatDate, useLanguage, useT } from "../i18n";
import type { MessageKey } from "../i18n/messages";
import { statusMessage } from "./LoginPage";

interface TokenFormData {
  name: string;
  projectScope: "All" | number[];
  scopes: Scope[];
  expiresAt: string | null;
}

export interface TokenFormState {
  name: string;
  scopeMode: "all" | "specified";
  specifiedIds: number[];
  scopes: Scope[];
  expiresAt: string;
}

export type ExpiryPreset = "week" | "month" | "half-year" | "year" | "never";

export const DEFAULT_SCOPES: readonly Scope[] = [
  "metadata:read",
  "artifacts:read",
  "artifacts:write",
];

export const EXPIRY_PRESETS: readonly ExpiryPreset[] = [
  "week",
  "month",
  "half-year",
  "year",
  "never",
];

export const EXPIRY_PRESET_DAYS: Record<ExpiryPreset, number | null> = {
  week: 7,
  month: 30,
  "half-year": 183,
  year: 365,
  never: null,
};

export function expiresAtForPreset(
  preset: ExpiryPreset,
  now: number = Date.now(),
): string | null {
  const days = EXPIRY_PRESET_DAYS[preset];
  return days === null ? null : new Date(now + days * 864e5).toISOString();
}

const EXPIRY_LABEL_KEYS: Record<ExpiryPreset, MessageKey> = {
  week: "tokens.form.expiryWeek",
  month: "tokens.form.expiryMonth",
  "half-year": "tokens.form.expiryHalfYear",
  year: "tokens.form.expiryYear",
  never: "tokens.form.noExpiry",
};

export function scopeErrorKey(form: TokenFormState): string | null {
  return form.scopeMode === "specified" && form.specifiedIds.length === 0
    ? "specified-empty"
    : null;
}

// 默认中文错误文案（既有单测按中文断言校验行为；页面改用 scopeErrorKey + i18n 文案）。
export function scopeError(form: TokenFormState): string | null {
  return scopeErrorKey(form) === null
    ? null
    : "「指定项目」模式至少选择一个项目，或切换为「全部项目」";
}

function scopeLabel(
  scope: ProjectScopeDto,
  projects: Project[],
  allLabel: string,
  specifiedLabel: string,
): string {
  if (scope === "All") {
    return allLabel;
  }
  const list = scope.Specified
    .map((id) => projects.find((project) => project.project_id === id)?.name ?? `#${id}`)
    .join(", ");
  return specifiedLabel.replace("{list}", list);
}

export function TokensPage({ client }: { client: ApiClient }): ReactElement {
  const t = useT();
  const { lang } = useLanguage();
  const [tokens, setTokens] = useState<TokenSummary[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editTarget, setEditTarget] = useState<TokenSummary | null>(null);
  const [rotateTarget, setRotateTarget] = useState<TokenSummary | null>(null);
  const [revokeTarget, setRevokeTarget] = useState<TokenSummary | null>(null);
  const [jwtReveal, setJwtReveal] = useState<{ jwt: string; label: string } | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      const [tokenList, projectList] = await Promise.all([
        withAuthRetry(sessionStore, (bearer) => client.listTokens(bearer)),
        withAuthRetry(sessionStore, (bearer) => client.listAllProjects(bearer)),
      ]);
      setTokens(tokenList);
      setProjects(projectList);
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }, [client]);

  useEffect(() => {
    void load();
  }, [load]);

  async function createToken(data: TokenFormData): Promise<void> {
    const issued = await withAuthRetry(sessionStore, (bearer) =>
      client.createToken(bearer, {
        name: data.name.trim(),
        project_scope: encodeProjectScope(data.projectScope === "All" ? "all" : data.projectScope),
        scopes: data.scopes,
        ...(data.expiresAt ? { expires_at: data.expiresAt } : {}),
      }),
    );
    setShowCreate(false);
    setJwtReveal({ jwt: issued.jwt, label: t("tokens.jwt.created") });
    await load();
  }

  async function saveToken(token: TokenSummary, data: TokenFormData): Promise<void> {
    const patch: TokenUpdateInput = {
      name: data.name.trim() || token.name,
      project_scope: encodeProjectScope(data.projectScope === "All" ? "all" : data.projectScope),
      scopes: data.scopes,
    };
    await withAuthRetry(sessionStore, (bearer) =>
      client.updateToken(bearer, token.token_id, patch),
    );
    setEditTarget(null);
    await load();
  }

  async function rotateToken(token: TokenSummary): Promise<void> {
    setBusy(true);
    setError("");
    setRotateTarget(null);
    try {
      const issued = await withAuthRetry(sessionStore, (bearer) =>
        client.rotateToken(bearer, token.token_id),
      );
      setJwtReveal({
        jwt: issued.jwt,
        label: t("tokens.jwt.resigned", { name: token.name }),
      });
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function revokeToken(token: TokenSummary): Promise<void> {
    setBusy(true);
    setError("");
    setRevokeTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.revokeToken(bearer, token.token_id),
      );
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <div className="page-head">
        <div className="page-title">
          <h1>{t("tokens.title")}</h1>
          <span className="count-badge">{tokens.length}</span>
        </div>
        <Btn size="sm" onClick={() => setShowCreate(true)}>
          <Icon name="plus" size={13} />
          {t("tokens.new")}
        </Btn>
      </div>

      {error && (
        <div className="page-pad-vertical">
          <ErrorBanner message={error} onDismiss={() => setError("")} />
        </div>
      )}

      <div className="table-wrap">
        <table className="data-table tokens-table">
          <thead>
            <tr>
              <th>{t("tokens.column.name")}</th>
              <th>{t("tokens.column.scope")}</th>
              <th>{t("tokens.column.permissions")}</th>
              <th>{t("tokens.column.created")}</th>
              <th className="th-right">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {tokens.map((token) => (
              <tr key={token.token_id}>
                <td>
                  <div className="token-name">{token.name}</div>
                  <div className="mono muted">{token.token_id}</div>
                </td>
                <td className="muted">
                  <span className={token.project_scope === "All" ? "strong" : "mono"}>
                    {scopeLabel(
                      token.project_scope,
                      projects,
                      t("tokens.allProjects"),
                      t("tokens.specified", { list: "{list}" }),
                    )}
                  </span>
                </td>
                <td>
                  <div className="badge-row">
                    {token.scopes.map((scope) => (
                      <Badge key={scope} variant="scope">{scope}</Badge>
                    ))}
                  </div>
                </td>
                <td className="mono muted whitespace-nowrap">
                  {formatDate(token.created_at, lang)}
                </td>
                <td className="td-right">
                  <div className="row-actions">
                    <Btn size="sm" variant="ghost" disabled={busy} onClick={() => setEditTarget(token)}>
                      <Icon name="edit" size={11} />
                      {t("tokens.edit")}
                    </Btn>
                    <Btn size="sm" variant="ghost" disabled={busy} onClick={() => setRotateTarget(token)}>
                      <Icon name="rotate" size={11} />
                      {t("tokens.resign")}
                    </Btn>
                    <Btn
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      aria-label={t("tokens.revoke")}
                      onClick={() => setRevokeTarget(token)}
                    >
                      <Icon name="trash" size={11} className="danger-icon" />
                    </Btn>
                  </div>
                </td>
              </tr>
            ))}
            {tokens.length === 0 && (
              <tr>
                <td colSpan={5} className="empty-cell">
                  {t("tokens.empty")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {showCreate && (
        <TokenFormModal
          title={t("tokens.createTitle")}
          projects={projects}
          onClose={() => setShowCreate(false)}
          onSave={(data) => createToken(data)}
        />
      )}
      {editTarget && (
        <TokenFormModal
          title={t("tokens.editTitle", { name: editTarget.name })}
          projects={projects}
          init={{
            name: editTarget.name,
            projectScope:
              editTarget.project_scope === "All"
                ? "All"
                : [...editTarget.project_scope.Specified],
            scopes: [...editTarget.scopes],
          }}
          isEdit
          onClose={() => setEditTarget(null)}
          onSave={(data) => saveToken(editTarget, data)}
        />
      )}
      {rotateTarget && (
        <Confirm
          title={t("tokens.resignTitle", { name: rotateTarget.name })}
          body={t("tokens.resignBody")}
          confirmLabel={t("tokens.resignConfirm")}
          onConfirm={() => { void rotateToken(rotateTarget); }}
          onCancel={() => setRotateTarget(null)}
          danger={false}
        />
      )}
      {revokeTarget && (
        <Confirm
          title={t("tokens.revokeTitle", { name: revokeTarget.name })}
          body={t("tokens.revokeBody")}
          confirmLabel={t("tokens.revokeConfirm")}
          onConfirm={() => { void revokeToken(revokeTarget); }}
          onCancel={() => setRevokeTarget(null)}
        />
      )}
      {jwtReveal && <JwtRevealModal jwt={jwtReveal.jwt} label={jwtReveal.label} onClose={() => setJwtReveal(null)} />}
    </div>
  );
}

function TokenFormModal({
  title,
  init,
  projects,
  onClose,
  onSave,
  isEdit = false,
}: {
  title: string;
  init?: { name: string; projectScope: "All" | number[]; scopes: Scope[] };
  projects: Project[];
  onClose: () => void;
  onSave: (data: TokenFormData) => Promise<void>;
  isEdit?: boolean;
}): ReactElement {
  const t = useT();
  const [name, setName] = useState(init?.name ?? "");
  const [scopeType, setScopeType] = useState<"all" | "specified">(
    Array.isArray(init?.projectScope) ? "specified" : "all",
  );
  const [specified, setSpecified] = useState<number[]>(
    Array.isArray(init?.projectScope) ? [...init.projectScope] : [],
  );
  const [scopes, setScopes] = useState<Scope[]>(
    init?.scopes ? [...init.scopes] : [...DEFAULT_SCOPES],
  );
  const [expiryPreset, setExpiryPreset] = useState<ExpiryPreset>(
    isEdit ? "never" : "month",
  );
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  function toggleProject(id: number): void {
    setSpecified((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  }

  function toggleScope(scope: Scope): void {
    setScopes((current) =>
      current.includes(scope) ? current.filter((item) => item !== scope) : [...current, scope],
    );
  }

  async function handleSave(): Promise<void> {
    const trimmed = name.trim();
    if (!trimmed) {
      setError(t("tokens.form.nameRequired"));
      return;
    }
    if (scopeType === "specified" && specified.length === 0) {
      setError(t("tokens.form.specifiedRequired"));
      return;
    }
    if (scopes.length === 0) {
      setError(t("tokens.form.scopesRequired"));
      return;
    }
    const expiresAt = expiresAtForPreset(expiryPreset);
    setError("");
    setSaving(true);
    try {
      await onSave({
        name: trimmed,
        projectScope: scopeType === "all" ? "All" : specified,
        scopes,
        expiresAt,
      });
    } catch (caught) {
      setError(statusMessage(caught));
      setSaving(false);
    }
  }

  return (
    <Modal title={title} onClose={onClose} wide>
      <div className="form-grid">
        <div className="form-col">
          <Field label={t("tokens.form.name")} htmlFor="token-name">
            <Inp
              id="token-name"
              placeholder={t("tokens.form.namePlaceholder")}
              value={name}
              onChange={(event) => { setName(event.target.value); setError(""); }}
              autoFocus
            />
          </Field>
          <Field label={t("tokens.form.scope")}>
            <div className="segmented">
              <button
                type="button"
                className={scopeType === "all" ? "active" : undefined}
                onClick={() => setScopeType("all")}
              >
                {t("tokens.form.all")}
              </button>
              <button
                type="button"
                className={scopeType === "specified" ? "active" : undefined}
                onClick={() => setScopeType("specified")}
              >
                {t("tokens.form.specified")}
              </button>
            </div>
            {scopeType === "specified" && (
              <div className="checkbox-list">
                {projects.map((project) => (
                  <label key={project.project_id} className="checkbox-row">
                    <input
                      type="checkbox"
                      checked={specified.includes(project.project_id)}
                      onChange={() => toggleProject(project.project_id)}
                    />
                    <span className="mono flex-1">{project.name}</span>
                    <Badge variant={project.visibility}>{project.visibility}</Badge>
                  </label>
                ))}
              </div>
            )}
          </Field>
          {isEdit ? (
            <p className="field-hint mono">{t("tokens.form.editNoResignHint")}</p>
          ) : (
            <Field label={t("tokens.form.expiry")}>
              <div className="segmented">
                {EXPIRY_PRESETS.map((preset) => (
                  <button
                    key={preset}
                    type="button"
                    className={expiryPreset === preset ? "active" : undefined}
                    onClick={() => { setExpiryPreset(preset); setError(""); }}
                  >
                    {t(EXPIRY_LABEL_KEYS[preset])}
                  </button>
                ))}
              </div>
            </Field>
          )}
        </div>
        <div className="form-col">
          <Field label={t("tokens.form.scopes")}>
            <div className="checkbox-list">
              <div className="scope-toggle-row">
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    aria-label={t("tokens.form.selectAll")}
                    checked={scopes.length === ALL_SCOPES.length}
                    onChange={() => setScopes([...ALL_SCOPES])}
                  />
                  <span>{t("tokens.form.selectAll")}</span>
                </label>
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    aria-label={t("tokens.form.selectNone")}
                    checked={scopes.length === 0}
                    onChange={() => setScopes([])}
                  />
                  <span>{t("tokens.form.selectNone")}</span>
                </label>
              </div>
              {ALL_SCOPES.map((scope) => (
                <label key={scope} className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={scopes.includes(scope)}
                    onChange={() => toggleScope(scope)}
                  />
                  <code className="mono">{scope}</code>
                </label>
              ))}
            </div>
          </Field>
        </div>
      </div>

      {error && <ErrorBanner message={error} onDismiss={() => setError("")} />}
      <div className="modal-actions">
        <Btn variant="outline" onClick={onClose}>{t("common.cancel")}</Btn>
        <Btn disabled={saving} onClick={() => { void handleSave(); }}>
          {isEdit
            ? <><Icon name="edit" size={12} />{t("tokens.save")}</>
            : <><Icon name="plus" size={12} />{t("tokens.createBtn")}</>}
        </Btn>
      </div>
    </Modal>
  );
}

function JwtRevealModal({
  jwt,
  label,
  onClose,
}: {
  jwt: string;
  label: string;
  onClose: () => void;
}): ReactElement {
  const t = useT();
  const [copied, setCopied] = useState(false);

  function copy(): void {
    void navigator.clipboard.writeText(jwt).catch(() => undefined);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 2000);
  }

  return (
    <Modal title={label} onClose={onClose}>
      <div className="warn-box">
        <Icon name="alert" size={13} />
        <div>
          <div className="warn-title">{t("tokens.jwt.warnTitle")}</div>
          <div className="warn-body">{t("tokens.jwt.warnBody")}</div>
        </div>
      </div>
      <div className="jwt-box">
        <code className="mono select-all">{jwt}</code>
      </div>
      <div className="modal-actions">
        <Btn variant="outline" onClick={copy}>
          {copied ? <Icon name="check" size={12} className="success-icon" /> : <Icon name="copy" size={12} />}
          {copied ? t("common.copied") : t("tokens.jwt.copy")}
        </Btn>
        <Btn onClick={onClose}>{t("tokens.jwt.done")}</Btn>
      </div>
    </Modal>
  );
}
