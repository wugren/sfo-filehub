import { useCallback, useEffect, useState, type ReactElement } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { ApiClient } from "../api/client";
import {
  formatBytes,
  type Collaborator,
  type Project,
  type ProjectRole,
  type VersionRecord,
  type Visibility,
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
  Sel,
} from "../components/ui";
import { formatDate, useLanguage, useT } from "../i18n";
import { saveBlob } from "../util/download";
import { statusMessage } from "./LoginPage";

type Tab = "versions" | "collaborators";

interface UploadState {
  app: string;
  file: File | null;
  busy: boolean;
}

export function ProjectDetailPage({ client }: { client: ApiClient }): ReactElement {
  const t = useT();
  const { lang } = useLanguage();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const projectId = Number(id);
  const [tab, setTab] = useState<Tab>(
    searchParams.get("tab") === "collaborators" ? "collaborators" : "versions",
  );
  const [project, setProject] = useState<Project | null>(null);
  const [versions, setVersions] = useState<VersionRecord[]>([]);
  const [collaborators, setCollaborators] = useState<Collaborator[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [visConfirm, setVisConfirm] = useState(false);
  const [showAddCollab, setShowAddCollab] = useState(false);
  const [removeTarget, setRemoveTarget] = useState<Collaborator | null>(null);
  const [newVersion, setNewVersion] = useState("");
  const [uploads, setUploads] = useState<Record<string, UploadState>>({});
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloaded, setDownloaded] = useState<string | null>(null);
  const [copiedSha, setCopiedSha] = useState<string | null>(null);
  const [lockTarget, setLockTarget] = useState<VersionRecord | null>(null);
  const [deleteAppTarget, setDeleteAppTarget] = useState<{ version: string; app: string } | null>(
    null,
  );

  const load = useCallback(async () => {
    if (!Number.isInteger(projectId) || projectId <= 0) {
      setError(t("projects.empty"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      const [projectList, versionList] = await Promise.all([
        withAuthRetry(sessionStore, (bearer) => client.listProjects(bearer)),
        withAuthRetry(sessionStore, (bearer) => client.listVersions(bearer, projectId)),
      ]);
      const found = projectList.find((item) => item.project_id === projectId) ?? null;
      setProject(found);
      setVersions(versionList);
      if (!found) {
        setError(statusMessage(new Error(t("notFound.title"))));
      }
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }, [client, projectId, t]);

  const loadCollaborators = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      const list = await withAuthRetry(sessionStore, (bearer) =>
        client.listCollaborators(bearer, projectId),
      );
      setCollaborators(list);
    } catch (caught) {
      setCollaborators([]);
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }, [client, projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (tab === "collaborators" && Number.isInteger(projectId) && projectId > 0) {
      void loadCollaborators();
    }
  }, [tab, projectId, loadCollaborators]);

  async function onCreateVersion(): Promise<void> {
    const version = newVersion.trim();
    if (!version) {
      setError(t("versions.createEmpty"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.createVersion(bearer, projectId, version),
      );
      setNewVersion("");
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onUploadApp(version: string): Promise<void> {
    const state = uploads[version] ?? { app: "", file: null, busy: false };
    const app = state.app.trim();
    if (!app) {
      setError(t("versions.appNameEmpty"));
      return;
    }
    if (!state.file) {
      setError(t("versions.fileEmpty"));
      return;
    }
    setUploads((current) => ({ ...current, [version]: { ...state, busy: true } }));
    setError("");
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.uploadApp(bearer, projectId, version, app, state.file as Blob),
      );
      setUploads((current) => ({ ...current, [version]: { app: "", file: null, busy: false } }));
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
      setUploads((current) => ({ ...current, [version]: { ...state, busy: false } }));
    }
  }

  async function onDownload(version: string, app: string): Promise<void> {
    const key = `${version}/${app}`;
    setError("");
    setDownloading(key);
    try {
      const blob = await withAuthRetry(sessionStore, (bearer) =>
        client.download(bearer, projectId, version, app),
      );
      saveBlob(blob, `${projectId}-${version}-${app}.tar.gz`);
      setDownloaded(key);
      window.setTimeout(() => {
        setDownloaded((current) => (current === key ? null : current));
      }, 2500);
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setDownloading(null);
    }
  }

  async function onCopySha256(key: string, sha256: string): Promise<void> {
    const writeText = navigator.clipboard?.writeText;
    if (!writeText) {
      return;
    }
    try {
      await writeText.call(navigator.clipboard, sha256);
      setCopiedSha(key);
      window.setTimeout(() => {
        setCopiedSha((current) => (current === key ? null : current));
      }, 2000);
    } catch {
      // 剪贴板权限失败时保留完整文本，用户仍可手动复制。
    }
  }

  async function onLockVersion(): Promise<void> {
    if (!lockTarget) {
      return;
    }
    const version = lockTarget.version;
    setBusy(true);
    setError("");
    setLockTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.lockVersion(bearer, projectId, version),
      );
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onDeleteApp(): Promise<void> {
    if (!deleteAppTarget) {
      return;
    }
    const { version, app } = deleteAppTarget;
    setBusy(true);
    setError("");
    setDeleteAppTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.deleteApp(bearer, projectId, version, app),
      );
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(): Promise<void> {
    setBusy(true);
    setError("");
    setDeleteConfirm(false);
    try {
      await withAuthRetry(sessionStore, (bearer) => client.deleteProject(bearer, projectId));
      navigate("/projects", { replace: true });
    } catch (caught) {
      setError(statusMessage(caught));
      setBusy(false);
    }
  }

  async function onToggleVisibility(): Promise<void> {
    if (!project) {
      return;
    }
    const next: Visibility = project.visibility === "private" ? "public" : "private";
    setBusy(true);
    setError("");
    setVisConfirm(false);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.setVisibility(bearer, projectId, next),
      );
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onAddCollab(userId: number, role: ProjectRole): Promise<void> {
    setBusy(true);
    setError("");
    setShowAddCollab(false);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.setCollaborator(bearer, projectId, userId, role),
      );
      await loadCollaborators();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onChangeRole(user: Collaborator, role: ProjectRole): Promise<void> {
    setBusy(true);
    setError("");
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.setCollaborator(bearer, projectId, user.user_id, role),
      );
      await loadCollaborators();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onRemoveCollab(user: Collaborator): Promise<void> {
    setBusy(true);
    setError("");
    setRemoveTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.removeCollaborator(bearer, projectId, user.user_id),
      );
      await loadCollaborators();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  const projectVersions = versions.filter((item) => item.project_id === projectId);

  return (
    <div className="page">
      <div className="page-head detail-head">
        <div className="breadcrumb">
          <button type="button" className="breadcrumb-link" onClick={() => navigate("/projects")}>
            <Icon name="arrowLeft" size={11} />
            {t("detail.back")}
          </button>
          <Icon name="chevronRight" size={11} />
          <span className="breadcrumb-current mono">{project?.name ?? `#${projectId}`}</span>
        </div>
        {project && (
          <div className="detail-title-row">
            <div className="detail-title">
              <h1 className="mono">{project.name}</h1>
              <Badge
                variant={project.visibility}
                icon={project.visibility === "public" ? "globe" : "lock"}
              >
                {project.visibility}
              </Badge>
              <span className="detail-owner mono">{t("nav.userId", { id: project.owner })}</span>
            </div>
            <div className="row-actions">
              <Btn size="sm" variant="ghost" disabled={busy} onClick={() => setVisConfirm(true)}>
                <Icon name={project.visibility === "public" ? "lock" : "globe"} size={11} />
                {project.visibility === "public" ? t("projects.makePrivate") : t("projects.makePublic")}
              </Btn>
              <Btn size="sm" variant="ghost" disabled={busy} onClick={() => setDeleteConfirm(true)}>
                <Icon name="trash" size={11} className="danger-icon" />
                {t("detail.delete")}
              </Btn>
            </div>
          </div>
        )}
        <p className="detail-meta mono">
          {t("detail.versionCount", { count: projectVersions.length })}
        </p>
      </div>

      {error && (
        <div className="page-pad-vertical">
          <ErrorBanner message={error} onDismiss={() => setError("")} />
        </div>
      )}

      <div className="tab-bar">
        {(["versions", "collaborators"] as const).map((value) => (
          <button
            key={value}
            type="button"
            className={tab === value ? "tab active" : "tab"}
            onClick={() => setTab(value)}
          >
            {value === "versions"
              ? `${t("detail.tabVersions")} (${projectVersions.length})`
              : t("detail.tabCollaborators")}
          </button>
        ))}
      </div>

      <div className="page-body">
        {tab === "versions" && (
          <VersionsTab
            versions={projectVersions}
            langLabel={lang}
            newVersion={newVersion}
            onNewVersionChange={setNewVersion}
            onCreateVersion={() => { void onCreateVersion(); }}
            uploads={uploads}
            onUploadStateChange={(version, state) =>
              setUploads((current) => ({ ...current, [version]: state }))
            }
            onUpload={(version) => { void onUploadApp(version); }}
            downloading={downloading}
            downloaded={downloaded}
            onDownload={(version, app) => { void onDownload(version, app); }}
            copiedSha={copiedSha}
            onCopySha256={(key, sha256) => { void onCopySha256(key, sha256); }}
            onLock={(version) => setLockTarget(version)}
            onDeleteApp={(version, app) => setDeleteAppTarget({ version, app })}
          />
        )}
        {tab === "collaborators" && project && (
          <CollaboratorsTab
            project={project}
            collaborators={collaborators}
            busy={busy}
            onAdd={() => setShowAddCollab(true)}
            onChangeRole={(user, role) => { void onChangeRole(user, role); }}
            onRemove={(user) => setRemoveTarget(user)}
          />
        )}
      </div>

      {deleteConfirm && project && (
        <Confirm
          title={t("projects.deleteTitle", { name: project.name })}
          body={t("projects.deleteBody")}
          confirmLabel={t("projects.deleteConfirm")}
          onConfirm={() => { void onDelete(); }}
          onCancel={() => setDeleteConfirm(false)}
        />
      )}
      {visConfirm && project && (
        <Confirm
          title={t("projects.visTitle")}
          body={t("projects.visBody", {
            name: project.name,
            from: project.visibility,
            to: project.visibility === "public" ? "private" : "public",
          })}
          confirmLabel={t("projects.visConfirm")}
          onConfirm={() => { void onToggleVisibility(); }}
          onCancel={() => setVisConfirm(false)}
          danger={false}
        />
      )}
      {showAddCollab && (
        <AddCollaboratorModal
          onClose={() => setShowAddCollab(false)}
          onAdd={(userId, role) => { void onAddCollab(userId, role); }}
        />
      )}
      {removeTarget && (
        <Confirm
          title={t("collab.removeTitle")}
          body={t("collab.removeBody", { id: removeTarget.user_id })}
          confirmLabel={t("collab.removeConfirm")}
          onConfirm={() => { void onRemoveCollab(removeTarget); }}
          onCancel={() => setRemoveTarget(null)}
        />
      )}
      {lockTarget && (
        <Confirm
          title={t("versions.lockTitle")}
          body={t("versions.lockBody", { version: lockTarget.version })}
          confirmLabel={t("versions.lockConfirm")}
          onConfirm={() => { void onLockVersion(); }}
          onCancel={() => setLockTarget(null)}
        />
      )}
      {deleteAppTarget && (
        <Confirm
          title={t("versions.deleteAppTitle")}
          body={t("versions.deleteAppBody", deleteAppTarget)}
          confirmLabel={t("versions.deleteAppConfirm")}
          onConfirm={() => { void onDeleteApp(); }}
          onCancel={() => setDeleteAppTarget(null)}
        />
      )}
    </div>
  );
}

function VersionsTab({
  versions,
  langLabel,
  newVersion,
  onNewVersionChange,
  onCreateVersion,
  uploads,
  onUploadStateChange,
  onUpload,
  downloading,
  downloaded,
  onDownload,
  copiedSha,
  onCopySha256,
  onLock,
  onDeleteApp,
}: {
  versions: VersionRecord[];
  langLabel: "zh" | "en";
  newVersion: string;
  onNewVersionChange: (value: string) => void;
  onCreateVersion: () => void;
  uploads: Record<string, UploadState>;
  onUploadStateChange: (version: string, state: UploadState) => void;
  onUpload: (version: string) => void;
  downloading: string | null;
  downloaded: string | null;
  onDownload: (version: string, app: string) => void;
  copiedSha: string | null;
  onCopySha256: (key: string, sha256: string) => void;
  onLock: (version: VersionRecord) => void;
  onDeleteApp: (version: string, app: string) => void;
}): ReactElement {
  const t = useT();
  return (
    <div className="versions-tab">
      <div className="version-create-row">
        <Inp
          className="version-create-input"
          placeholder={t("versions.createPlaceholder")}
          value={newVersion}
          onChange={(event) => onNewVersionChange(event.target.value)}
        />
        <Btn size="sm" variant="outline" disabled={!newVersion.trim()} onClick={onCreateVersion}>
          <Icon name="plus" size={12} />
          {t("versions.create")}
        </Btn>
      </div>

      {versions.map((record) => {
        const locked = record.locked_at !== null;
        const upload = uploads[record.version] ?? { app: "", file: null, busy: false };
        return (
          <div key={record.version} className="version-card">
            <div className="version-card-head">
              <div className="version-card-title">
                <span className="mono version-name">{record.version}</span>
                {locked && <Badge variant="danger">{t("versions.locked")}</Badge>}
                <span className="mono muted">{formatDate(record.published_at, langLabel)}</span>
              </div>
              {!locked && (
                <Btn size="sm" variant="ghost" onClick={() => onLock(record)}>
                  <Icon name="lock" size={11} />
                  {t("versions.lock")}
                </Btn>
              )}
            </div>

            <div className="table-wrap">
              <table className="data-table version-table">
                <colgroup>
                  <col className="version-col-app" />
                  <col className="version-col-size" />
                  <col className="version-col-sha" />
                  <col className="version-col-updated" />
                  <col className="version-col-actions" />
                </colgroup>
                <thead>
                  <tr>
                    <th>{t("versions.column.app")}</th>
                    <th>{t("versions.column.size")}</th>
                    <th>{t("versions.column.sha")}</th>
                    <th>{t("versions.column.updated")}</th>
                    <th className="th-right">{t("common.actions")}</th>
                  </tr>
                </thead>
                <tbody>
                  {record.apps.map((app) => {
                    const key = `${record.version}/${app.app}`;
                    return (
                      <tr key={app.app}>
                        <td className="mono">{app.app}</td>
                        <td className="mono muted">{formatBytes(app.size)}</td>
                        <td>
                          <div className="sha-cell">
                            <span className="mono muted sha-value">{app.sha256}</span>
                            <Btn
                              size="sm"
                              variant={copiedSha === key ? "outline" : "ghost"}
                              className="sha-copy-btn"
                              aria-label={copiedSha === key ? t("common.copied") : t("common.copy")}
                              title={copiedSha === key ? t("common.copied") : t("common.copy")}
                              onClick={() => onCopySha256(key, app.sha256)}
                            >
                              <Icon name={copiedSha === key ? "check" : "copy"} size={11} />
                            </Btn>
                          </div>
                        </td>
                        <td className="mono muted">{formatDate(app.updated_at, langLabel)}</td>
                        <td>
                          <div className="version-actions">
                            <Btn
                              size="sm"
                              variant={downloaded === key ? "outline" : "ghost"}
                              disabled={downloading === key}
                              onClick={() => onDownload(record.version, app.app)}
                            >
                              {downloaded === key
                                ? <Icon name="check" size={11} className="success-icon" />
                                : <Icon name="download" size={11} />}
                              {downloading === key
                                ? t("versions.downloading")
                                : downloaded === key
                                  ? t("versions.done")
                                  : t("versions.download")}
                            </Btn>
                            {!locked && (
                              <Btn
                                size="sm"
                                variant="ghost"
                                aria-label={t("versions.deleteAppConfirm")}
                                onClick={() => onDeleteApp(record.version, app.app)}
                              >
                                <Icon name="trash" size={11} className="danger-icon" />
                              </Btn>
                            )}
                            {locked && <span className="version-delete-slot" aria-hidden="true" />}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                  {record.apps.length === 0 && (
                    <tr>
                      <td colSpan={5} className="empty-cell">
                        {t("versions.noApps")}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {!locked && (
              <div className="version-upload-row">
                <Inp
                  className="version-app-input"
                  placeholder={t("versions.appName")}
                  value={upload.app}
                  onChange={(event) =>
                    onUploadStateChange(record.version, { ...upload, app: event.target.value })
                  }
                />
                <input
                  type="file"
                  accept=".tar.gz,application/gzip"
                  onChange={(event) =>
                    onUploadStateChange(record.version, {
                      ...upload,
                      file: event.target.files?.[0] ?? null,
                    })
                  }
                />
                <Btn
                  size="sm"
                  variant="outline"
                  disabled={upload.busy || !upload.app.trim() || !upload.file}
                  onClick={() => onUpload(record.version)}
                >
                  <Icon name="upload" size={12} />
                  {upload.busy ? t("versions.uploading") : t("versions.uploadApp")}
                </Btn>
              </div>
            )}
          </div>
        );
      })}
      {versions.length === 0 && (
        <div className="table-wrap">
          <table className="data-table">
            <tbody>
              <tr>
                <td className="empty-cell">{t("versions.empty")}</td>
              </tr>
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function CollaboratorsTab({
  project,
  collaborators,
  busy,
  onAdd,
  onChangeRole,
  onRemove,
}: {
  project: Project;
  collaborators: Collaborator[];
  busy: boolean;
  onAdd: () => void;
  onChangeRole: (user: Collaborator, role: ProjectRole) => void;
  onRemove: (user: Collaborator) => void;
}): ReactElement {
  const t = useT();
  return (
    <div className="collab-tab">
      <div className="collab-head">
        <p className="muted mono">{t("collab.ownerHint", { id: project.owner })}</p>
        <Btn size="sm" variant="outline" onClick={onAdd}>
          <Icon name="plus" size={12} />
          {t("collab.add")}
        </Btn>
      </div>
      <div className="table-wrap">
        <table className="data-table">
          <thead>
            <tr>
              <th>{t("collab.column.userId")}</th>
              <th>{t("collab.column.role")}</th>
              <th className="th-right">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {collaborators.map((user) => (
              <tr key={user.user_id}>
                <td className="mono">{user.user_id}</td>
                <td>
                  <Sel
                    className="role-select"
                    value={user.role}
                    disabled={busy}
                    onChange={(event) =>
                      onChangeRole(user, event.target.value as ProjectRole)
                    }
                  >
                    {(["read", "write", "admin"] as const).map((role) => (
                      <option key={role} value={role}>{role}</option>
                    ))}
                  </Sel>
                </td>
                <td className="td-right">
                  <Btn
                    size="sm"
                    variant="ghost"
                    disabled={busy}
                    aria-label={t("collab.removeConfirm")}
                    onClick={() => onRemove(user)}
                  >
                    <Icon name="trash" size={11} className="danger-icon" />
                  </Btn>
                </td>
              </tr>
            ))}
            {collaborators.length === 0 && (
              <tr>
                <td colSpan={3} className="empty-cell">
                  {t("collab.empty")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function AddCollaboratorModal({
  onClose,
  onAdd,
}: {
  onClose: () => void;
  onAdd: (userId: number, role: ProjectRole) => void;
}): ReactElement {
  const t = useT();
  const [userId, setUserId] = useState("");
  const [role, setRole] = useState<ProjectRole>("read");
  const [error, setError] = useState("");

  function submit(): void {
    const parsed = Number(userId);
    if (!Number.isInteger(parsed) || parsed <= 0) {
      setError(t("collab.userIdInvalid"));
      return;
    }
    onAdd(parsed, role);
  }

  return (
    <Modal title={t("collab.addTitle")} onClose={onClose}>
      <Field label={t("collab.userIdLabel")} hint={t("collab.userIdHint")}>
        <Inp
          value={userId}
          inputMode="numeric"
          placeholder="1"
          onChange={(event) => setUserId(event.target.value)}
        />
      </Field>
      <Field label={t("collab.roleLabel")}>
        <Sel value={role} onChange={(event) => setRole(event.target.value as ProjectRole)}>
          <option value="read">read</option>
          <option value="write">write</option>
          <option value="admin">admin</option>
        </Sel>
      </Field>
      {error && <ErrorBanner message={error} onDismiss={() => setError("")} />}
      <div className="modal-actions">
        <Btn variant="ghost" onClick={onClose}>{t("common.cancel")}</Btn>
        <Btn variant="primary" onClick={submit}>{t("collab.add")}</Btn>
      </div>
    </Modal>
  );
}
