import { useCallback, useEffect, useState, type ReactElement } from "react";
import { useNavigate } from "react-router-dom";
import { ApiClient } from "../api/client";
import type { Project, Visibility } from "../api/contract";
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
import { useT } from "../i18n";
import { statusMessage } from "./LoginPage";

const NAME_RE = /^[a-z0-9][a-z0-9_-]*$/;

export function ProjectsPage({ client }: { client: ApiClient }): ReactElement {
  const t = useT();
  const navigate = useNavigate();
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Project | null>(null);
  const [visTarget, setVisTarget] = useState<Project | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      const list = await withAuthRetry(sessionStore, (bearer) => client.listProjects(bearer));
      setProjects(list);
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }, [client]);

  useEffect(() => {
    void load();
  }, [load]);

  async function onCreate(name: string, visibility: Visibility): Promise<void> {
    setBusy(true);
    setError("");
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.createProject(bearer, name.trim(), visibility),
      );
      setShowCreate(false);
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(project: Project): Promise<void> {
    setBusy(true);
    setError("");
    setDeleteTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) => client.deleteProject(bearer, project.project_id));
      await load();
    } catch (caught) {
      setError(statusMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function onToggleVisibility(project: Project): Promise<void> {
    const next: Visibility = project.visibility === "private" ? "public" : "private";
    setBusy(true);
    setError("");
    setVisTarget(null);
    try {
      await withAuthRetry(sessionStore, (bearer) =>
        client.setVisibility(bearer, project.project_id, next),
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
          <h1>{t("projects.title")}</h1>
          <span className="count-badge">{projects.length}</span>
        </div>
        <Btn size="sm" onClick={() => setShowCreate(true)}>
          <Icon name="plus" size={13} />
          {t("projects.new")}
        </Btn>
      </div>

      {error && (
        <div className="page-pad-vertical">
          <ErrorBanner message={error} onDismiss={() => setError("")} />
        </div>
      )}

      <div className="table-wrap">
        <table className="data-table">
          <thead>
            <tr>
              <th>{t("projects.name")}</th>
              <th>{t("projects.visibility")}</th>
              <th>{t("projects.owner")}</th>
              <th className="th-right">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {projects.map((project) => (
              <tr
                key={project.project_id}
                className="clickable"
                onClick={() => navigate(`/projects/${project.project_id}`)}
              >
                <td>
                  <span className="mono project-name">{project.name}</span>
                </td>
                <td>
                  <Badge
                    variant={project.visibility}
                    icon={project.visibility === "public" ? "globe" : "lock"}
                  >
                    {project.visibility}
                  </Badge>
                </td>
                <td className="mono muted">{project.owner}</td>
                <td className="td-right">
                  <div className="row-actions" onClick={(event) => event.stopPropagation()}>
                    <Btn
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => setVisTarget(project)}
                    >
                      <Icon name={project.visibility === "public" ? "lock" : "globe"} size={11} />
                      {project.visibility === "public" ? t("projects.makePrivate") : t("projects.makePublic")}
                    </Btn>
                    <Btn
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      aria-label={t("projects.deleteConfirm")}
                      onClick={() => setDeleteTarget(project)}
                    >
                      <Icon name="trash" size={11} className="danger-icon" />
                    </Btn>
                  </div>
                </td>
              </tr>
            ))}
            {projects.length === 0 && (
              <tr>
                <td colSpan={4} className="empty-cell">
                  {t("projects.empty")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {showCreate && (
        <CreateProjectModal
          onClose={() => setShowCreate(false)}
          onCreate={(name, visibility) => { void onCreate(name, visibility); }}
        />
      )}
      {deleteTarget && (
        <Confirm
          title={t("projects.deleteTitle", { name: deleteTarget.name })}
          body={t("projects.deleteBody")}
          confirmLabel={t("projects.deleteConfirm")}
          onConfirm={() => { void onDelete(deleteTarget); }}
          onCancel={() => setDeleteTarget(null)}
        />
      )}
      {visTarget && (
        <Confirm
          title={t("projects.visTitle")}
          body={t("projects.visBody", {
            name: visTarget.name,
            from: visTarget.visibility,
            to: visTarget.visibility === "public" ? "private" : "public",
          })}
          confirmLabel={t("projects.visConfirm")}
          onConfirm={() => { void onToggleVisibility(visTarget); }}
          onCancel={() => setVisTarget(null)}
          danger={false}
        />
      )}
    </div>
  );
}

function CreateProjectModal({
  onClose,
  onCreate,
}: {
  onClose: () => void;
  onCreate: (name: string, visibility: Visibility) => void;
}): ReactElement {
  const t = useT();
  const [name, setName] = useState("");
  const [visibility, setVisibility] = useState<Visibility>("public");
  const [error, setError] = useState("");

  function handle() {
    const trimmed = name.trim();
    if (!trimmed) {
      setError(t("projects.createNameRequired"));
      return;
    }
    if (!NAME_RE.test(trimmed)) {
      setError(t("projects.createNameFormat"));
      return;
    }
    onCreate(trimmed, visibility);
  }

  return (
    <Modal title={t("projects.createTitle")} onClose={onClose}>
      <Field label={t("projects.name")} htmlFor="project-name">
        <Inp
          id="project-name"
          type="text"
          placeholder={t("projects.createNamePlaceholder")}
          value={name}
          onChange={(event) => { setName(event.target.value); setError(""); }}
          autoFocus
        />
      </Field>
      <Field label={t("projects.visibility")}>
        <div className="segmented">
          {(["public", "private"] as const).map((value) => (
            <button
              key={value}
              type="button"
              className={visibility === value ? "active" : undefined}
              onClick={() => setVisibility(value)}
            >
              <Icon name={value === "public" ? "globe" : "lock"} size={13} />
              {value}
            </button>
          ))}
        </div>
      </Field>
      {error && <ErrorBanner message={error} onDismiss={() => setError("")} />}
      <div className="modal-actions">
        <Btn variant="outline" onClick={onClose}>{t("common.cancel")}</Btn>
        <Btn onClick={handle}>
          <Icon name="plus" size={13} />
          {t("common.create")}
        </Btn>
      </div>
    </Modal>
  );
}
