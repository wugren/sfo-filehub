import { useState } from "react";
import {
  LogOut, Package, Key, Plus, Trash2, Download,
  RotateCcw, Edit3, X, AlertTriangle, Copy, Check,
  Users, Globe, Lock, ArrowLeft, Terminal, ChevronRight,
} from "lucide-react";

// ── Types ────────────────────────────────────────────────────────────────────

type View = "projects" | "project-detail" | "tokens";

interface Project {
  project_id: number;
  name: string;
  visibility: "public" | "private";
  owner: number;
}

interface VersionRecord {
  project_id: number;
  version: string;
  file_id: string;
  sha256: string;
  size: number;
  published_at: string;
}

interface TokenSummary {
  token_id: string;
  name: string;
  project_scope: "All" | { Specified: number[] };
  scopes: string[];
  created_at: string;
  updated_at: string;
}

interface Collaborator {
  user_id: number;
  role: "read" | "write" | "admin";
}

interface User {
  id: number;
  name: string;
}

// ── Seed Data ────────────────────────────────────────────────────────────────

const SEED_PROJECTS: Project[] = [
  { project_id: 1, name: "filehub-server",       visibility: "public",  owner: 1001 },
  { project_id: 2, name: "filehub-cli",           visibility: "public",  owner: 1001 },
  { project_id: 3, name: "internal-dashboard",    visibility: "private", owner: 1001 },
  { project_id: 4, name: "analytics-pipeline",    visibility: "private", owner: 1042 },
];

const SEED_VERSIONS: VersionRecord[] = [
  { project_id: 1, version: "v0.5.2", file_id: "f8e3a2b1", sha256: "a3f9c2e8d1b7f4a0c6e2d8b4f1a7c3e9", size: 18_432_512, published_at: "2026-08-19T14:32:00Z" },
  { project_id: 1, version: "v0.5.1", file_id: "c7d4e1a0", sha256: "b4f0d3e9c1a8f5b2e7d4c0a6f3d8b1e5", size: 18_210_048, published_at: "2026-08-12T09:15:00Z" },
  { project_id: 1, version: "v0.5.0", file_id: "a1b2c3d4", sha256: "c5e1f8d2b9a6c3e0f7d4a1b8e5c2f9d6", size: 17_895_424, published_at: "2026-08-01T16:00:00Z" },
  { project_id: 1, version: "v0.4.3", file_id: "e5f6a7b8", sha256: "d6f2a9e3c0b7d4f1a8e5b2d9f6a3b0e7", size: 16_640_000, published_at: "2026-07-15T11:45:00Z" },
  { project_id: 2, version: "v1.2.0", file_id: "b9c0d1e2", sha256: "e7a3f0d6c2b9a5f1d8e4b0c7f3d9a6b2", size: 9_437_184,  published_at: "2026-08-18T10:00:00Z" },
  { project_id: 2, version: "v1.1.5", file_id: "a0b1c2d3", sha256: "f8b4c1d7e3a0f6b2d9c5a1e8b5d2f9c6", size: 9_175_040,  published_at: "2026-08-05T08:30:00Z" },
  { project_id: 3, version: "v0.1.0", file_id: "c1d2e3f4", sha256: "a9d5b2e8f4a1d7b3e0c6f2a8d4b1e7f3", size: 24_117_248, published_at: "2026-07-28T12:00:00Z" },
  { project_id: 4, version: "v2.0.1", file_id: "d2e3f4a5", sha256: "b0e6c3a9f5b2d8e4a0f7b3c9e6a2d8f4", size: 31_457_280, published_at: "2026-08-17T17:00:00Z" },
];

const SEED_TOKENS: TokenSummary[] = [
  { token_id: "tok_8f3a2b1c", name: "CI/CD Pipeline",    project_scope: "All",                    scopes: ["artifacts:read", "artifacts:write", "metadata:read"], created_at: "2026-07-01T00:00:00Z", updated_at: "2026-07-01T00:00:00Z" },
  { token_id: "tok_c4d5e6f7", name: "Deploy Automation", project_scope: { Specified: [1, 2] },    scopes: ["artifacts:read", "metadata:read"],                   created_at: "2026-08-10T00:00:00Z", updated_at: "2026-08-10T00:00:00Z" },
  { token_id: "tok_a1b2c3d4", name: "Release Bot",        project_scope: "All",                    scopes: ["artifacts:write", "projects:create"],               created_at: "2026-08-15T00:00:00Z", updated_at: "2026-08-15T00:00:00Z" },
];

const SEED_COLLABORATORS: Record<number, Collaborator[]> = {
  1: [{ user_id: 1042, role: "admin" }, { user_id: 1087, role: "write" }, { user_id: 1156, role: "read" }],
  2: [{ user_id: 1042, role: "write" }],
  3: [],
  4: [{ user_id: 1001, role: "admin" }, { user_id: 1089, role: "read" }],
};

const ALL_SCOPES = [
  "metadata:read",
  "artifacts:read",
  "artifacts:write",
  "administration",
  "projects:create",
  "projects:delete",
] as const;

// ── Helpers ──────────────────────────────────────────────────────────────────

function fmtSize(b: number) {
  if (b >= 1048576) return `${(b / 1048576).toFixed(1)} MB`;
  if (b >= 1024)    return `${(b / 1024).toFixed(0)} KB`;
  return `${b} B`;
}

function fmtDate(iso: string) {
  return new Date(iso).toLocaleDateString("en-US", { year: "numeric", month: "short", day: "numeric" });
}

function uid() {
  return Math.random().toString(36).slice(2, 10);
}

function mockJwt() {
  const h = btoa(JSON.stringify({ alg: "EdDSA", typ: "JWT" })).replace(/=/g, "");
  const p = btoa(JSON.stringify({ sub: "1001", iat: Math.floor(Date.now() / 1000) })).replace(/=/g, "");
  return `${h}.${p}.${uid()}${uid()}${uid()}${uid()}`;
}

function scopeLabel(scope: "All" | { Specified: number[] }, projects: Project[]) {
  if (scope === "All") return "All projects";
  return scope.Specified.map(id => projects.find(p => p.project_id === id)?.name ?? `#${id}`).join(", ");
}

// ── Primitives ───────────────────────────────────────────────────────────────

type BV = "public" | "private" | "scope" | "neutral" | "role-read" | "role-write" | "role-admin";

function Badge({ children, variant = "neutral" }: { children: React.ReactNode; variant?: BV }) {
  const map: Record<BV, string> = {
    neutral:      "bg-white/5 text-muted-foreground",
    public:       "bg-emerald-950/80 text-emerald-400 ring-1 ring-emerald-800/50",
    private:      "bg-amber-950/80 text-amber-400 ring-1 ring-amber-800/50",
    scope:        "bg-primary/10 text-primary ring-1 ring-primary/25",
    "role-read":  "bg-white/5 text-slate-400",
    "role-write": "bg-sky-950/80 text-sky-400 ring-1 ring-sky-800/50",
    "role-admin": "bg-violet-950/80 text-violet-400 ring-1 ring-violet-800/50",
  };
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-mono leading-tight ${map[variant]}`}>
      {children}
    </span>
  );
}

function Btn({
  children, onClick, variant = "primary", size = "md", disabled = false, className = "", type = "button",
}: {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: "primary" | "ghost" | "danger" | "outline";
  size?: "sm" | "md";
  disabled?: boolean;
  className?: string;
  type?: "button" | "submit";
}) {
  const base = "inline-flex items-center gap-1.5 font-medium rounded-md transition-all duration-150 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/60";
  const sz = { sm: "px-2.5 py-1 text-xs", md: "px-3.5 py-1.5 text-sm" };
  const vs = {
    primary: "bg-primary text-primary-foreground hover:bg-primary/85 active:scale-95",
    ghost:   "text-muted-foreground hover:text-foreground hover:bg-white/5",
    danger:  "bg-destructive/90 text-white hover:bg-destructive active:scale-95",
    outline: "border border-border text-foreground hover:bg-white/5 active:scale-95",
  };
  return (
    <button type={type} className={`${base} ${sz[size]} ${vs[variant]} ${className}`} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  );
}

function Inp(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`w-full bg-black/25 border border-border rounded-md px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary/40 transition ${props.className ?? ""}`}
    />
  );
}

function Sel({ children, className = "", ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={`bg-muted border border-border rounded-md px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary/40 transition ${className}`}
    >
      {children}
    </select>
  );
}

function Lbl({ children }: { children: React.ReactNode }) {
  return <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-1.5">{children}</p>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <div className="mb-4"><Lbl>{label}</Lbl>{children}</div>;
}

function Modal({ title, children, onClose, wide = false }: {
  title: string; children: React.ReactNode; onClose: () => void; wide?: boolean;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className={`bg-card border border-border rounded-xl shadow-2xl w-full ${wide ? "max-w-2xl" : "max-w-lg"} max-h-[90vh] flex flex-col`}>
        <div className="flex items-center justify-between px-5 py-4 border-b border-border shrink-0">
          <h2 className="text-sm font-semibold text-foreground">{title}</h2>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground transition p-0.5 rounded">
            <X size={14} />
          </button>
        </div>
        <div className="p-5 overflow-y-auto">{children}</div>
      </div>
    </div>
  );
}

function Confirm({ title, body, confirmLabel, onConfirm, onCancel, danger = true }: {
  title: string; body: string; confirmLabel: string;
  onConfirm: () => void; onCancel: () => void; danger?: boolean;
}) {
  return (
    <Modal title={title} onClose={onCancel}>
      <p className="text-sm text-muted-foreground mb-5 leading-relaxed">{body}</p>
      <div className="flex gap-2 justify-end">
        <Btn variant="outline" onClick={onCancel}>Cancel</Btn>
        <Btn variant={danger ? "danger" : "primary"} onClick={onConfirm}>{confirmLabel}</Btn>
      </div>
    </Modal>
  );
}

function Err({ msg, onDismiss }: { msg: string; onDismiss: () => void }) {
  return (
    <div className="flex items-start gap-2 bg-destructive/10 border border-destructive/30 text-destructive rounded-md p-3 text-xs">
      <AlertTriangle size={13} className="shrink-0 mt-0.5" />
      <span className="flex-1">{msg}</span>
      <button onClick={onDismiss}><X size={12} /></button>
    </div>
  );
}

// ── Login ────────────────────────────────────────────────────────────────────

function LoginPage({ onLogin }: { onLogin: (user: User) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading]   = useState(false);
  const [error, setError]       = useState("");

  function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    if (!username || !password) { setError("Username and password are required."); return; }
    setLoading(true);
    setTimeout(() => {
      setLoading(false);
      if (username === "admin" || username === "demo") {
        onLogin({ id: 1001, name: username });
      } else {
        // simulates sfo-http: { err: 1, msg: "...", result: null }
        setError("Invalid credentials (err: 1). Hint: use username “admin”.");
      }
    }, 600);
  }

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <div className="w-full max-w-sm">
        <div className="mb-8 text-center">
          <div className="inline-flex items-center gap-2 mb-3">
            <Terminal size={18} className="text-primary" />
            <span className="text-base font-bold font-mono tracking-tight text-foreground">filehub</span>
          </div>
          <p className="text-sm text-muted-foreground">Sign in to your workspace</p>
        </div>

        <div className="bg-card border border-border rounded-xl p-6 shadow-2xl">
          <form onSubmit={submit}>
            <Field label="Username">
              <Inp
                type="text"
                placeholder="admin"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                autoComplete="username"
                autoFocus
              />
            </Field>
            <Field label="Password">
              <Inp
                type="password"
                placeholder="••••••••"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
              />
            </Field>
            {error && <div className="mb-4"><Err msg={error} onDismiss={() => setError("")} /></div>}
            <Btn type="submit" variant="primary" disabled={loading} className="w-full justify-center">
              {loading ? "Signing in…" : "Sign in"}
            </Btn>
          </form>
        </div>

        <p className="text-center text-xs text-muted-foreground mt-4 font-mono">
          Credentials stored in memory — not persisted
        </p>
      </div>
    </div>
  );
}

// ── Sidebar ──────────────────────────────────────────────────────────────────

function Sidebar({ user, view, onNavigate, onLogout }: {
  user: User; view: View;
  onNavigate: (v: "projects" | "tokens") => void;
  onLogout: () => void;
}) {
  const nav = [
    { id: "projects" as const, label: "Projects",   icon: <Package size={14} /> },
    { id: "tokens"   as const, label: "API Tokens",  icon: <Key size={14} /> },
  ];

  return (
    <aside className="w-52 shrink-0 bg-sidebar border-r border-sidebar-border flex flex-col h-full">
      <div className="px-4 py-4 border-b border-sidebar-border">
        <div className="flex items-center gap-2">
          <Terminal size={15} className="text-primary" />
          <span className="font-mono font-bold text-sm text-foreground tracking-tight">filehub</span>
        </div>
      </div>

      <nav className="flex-1 p-2 space-y-0.5 overflow-y-auto">
        {nav.map(item => {
          const active = view === item.id || (item.id === "projects" && view === "project-detail");
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm text-left transition-all duration-150 ${
                active
                  ? "bg-primary/10 text-primary font-medium"
                  : "text-muted-foreground hover:text-foreground hover:bg-white/5"
              }`}
            >
              {item.icon}
              {item.label}
            </button>
          );
        })}
      </nav>

      <div className="p-3 border-t border-sidebar-border">
        <div className="flex items-center gap-2.5 px-2 py-1.5 mb-1">
          <div className="w-6 h-6 rounded-full bg-primary/20 flex items-center justify-center text-primary text-xs font-bold font-mono shrink-0">
            {user.name[0].toUpperCase()}
          </div>
          <div className="min-w-0">
            <div className="text-xs font-medium text-foreground font-mono truncate">{user.name}</div>
            <div className="text-xs text-muted-foreground font-mono">id:{user.id}</div>
          </div>
        </div>
        <button
          onClick={onLogout}
          className="w-full flex items-center gap-2 px-3 py-1.5 rounded-md text-xs text-muted-foreground hover:text-foreground hover:bg-white/5 transition"
        >
          <LogOut size={12} /> Sign out
        </button>
      </div>
    </aside>
  );
}

// ── Projects Page ─────────────────────────────────────────────────────────────

function CreateProjectModal({ onClose, onCreate }: {
  onClose: () => void;
  onCreate: (name: string, visibility: "public" | "private") => void;
}) {
  const [name, setName]               = useState("");
  const [visibility, setVisibility]   = useState<"public" | "private">("public");
  const [error, setError]             = useState("");

  function handle() {
    if (!name.trim()) { setError("Project name is required."); return; }
    if (!/^[a-z0-9][a-z0-9_-]*$/.test(name.trim())) {
      setError("Use lowercase letters, digits, hyphens, or underscores."); return;
    }
    onCreate(name.trim(), visibility);
    onClose();
  }

  return (
    <Modal title="New Project" onClose={onClose}>
      <Field label="Name">
        <Inp
          type="text"
          placeholder="my-project"
          value={name}
          onChange={(e) => { setName(e.target.value); setError(""); }}
          autoFocus
        />
      </Field>
      <Field label="Visibility">
        <div className="flex gap-2">
          {(["public", "private"] as const).map(v => (
            <button
              key={v}
              type="button"
              onClick={() => setVisibility(v)}
              className={`flex-1 flex items-center justify-center gap-1.5 py-2 rounded-md text-sm border transition ${
                visibility === v
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:border-white/20 hover:text-foreground"
              }`}
            >
              {v === "public" ? <Globe size={13} /> : <Lock size={13} />}
              {v}
            </button>
          ))}
        </div>
      </Field>
      {error && <div className="mb-4"><Err msg={error} onDismiss={() => setError("")} /></div>}
      <div className="flex gap-2 justify-end mt-2">
        <Btn variant="outline" onClick={onClose}>Cancel</Btn>
        <Btn variant="primary" onClick={handle}><Plus size={13} />Create</Btn>
      </div>
    </Modal>
  );
}

function ProjectsPage({ projects, user, onSelect, onCreate, onDelete, onToggleVis }: {
  projects: Project[];
  user: User;
  onSelect: (p: Project) => void;
  onCreate: (name: string, vis: "public" | "private") => void;
  onDelete: (id: number) => void;
  onToggleVis: (id: number) => void;
}) {
  const [showCreate, setShowCreate]     = useState(false);
  const [delConfirm, setDelConfirm]     = useState<Project | null>(null);
  const [visConfirm, setVisConfirm]     = useState<Project | null>(null);
  const [error403, setError403]         = useState("");

  function guardDelete(p: Project) {
    if (p.owner !== user.id) {
      setError403(`403 Forbidden — projects:delete scope required. (owner: ${p.owner}, you: ${user.id})`);
    } else {
      setDelConfirm(p);
    }
  }

  function guardToggle(p: Project) {
    if (p.owner !== user.id) {
      setError403(`403 Forbidden — administration scope required. (owner: ${p.owner}, you: ${user.id})`);
    } else {
      setVisConfirm(p);
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between shrink-0">
        <div className="flex items-center gap-3">
          <h1 className="text-sm font-semibold text-foreground">Visible Projects</h1>
          <span className="font-mono text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">{projects.length}</span>
        </div>
        <Btn variant="primary" size="sm" onClick={() => setShowCreate(true)}>
          <Plus size={13} />New Project
        </Btn>
      </div>

      {error403 && (
        <div className="mx-6 mt-4">
          <Err msg={error403} onDismiss={() => setError403("")} />
        </div>
      )}

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm min-w-[600px]">
          <thead className="border-b border-border sticky top-0 bg-background z-10">
            <tr>
              <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Name</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Visibility</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Owner</th>
              <th className="text-right px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Actions</th>
            </tr>
          </thead>
          <tbody>
            {projects.map(p => (
              <tr
                key={p.project_id}
                className="border-b border-border/50 hover:bg-white/[0.03] transition-colors cursor-pointer group"
                onClick={() => onSelect(p)}
              >
                <td className="px-6 py-3">
                  <span className="font-mono text-sm text-foreground group-hover:text-primary transition-colors">{p.name}</span>
                </td>
                <td className="px-4 py-3">
                  <Badge variant={p.visibility}>
                    {p.visibility === "public" ? <Globe size={9} /> : <Lock size={9} />}
                    {p.visibility}
                  </Badge>
                </td>
                <td className="px-4 py-3 font-mono text-xs text-muted-foreground">{p.owner}</td>
                <td className="px-6 py-3" onClick={e => e.stopPropagation()}>
                  <div className="flex items-center gap-1 justify-end">
                    <Btn size="sm" variant="ghost" onClick={() => guardToggle(p)}>
                      {p.visibility === "public" ? <Lock size={11} /> : <Globe size={11} />}
                      {p.visibility === "public" ? "Make private" : "Make public"}
                    </Btn>
                    <Btn size="sm" variant="ghost" onClick={() => guardDelete(p)}>
                      <Trash2 size={11} className="text-destructive" />
                    </Btn>
                  </div>
                </td>
              </tr>
            ))}
            {projects.length === 0 && (
              <tr>
                <td colSpan={4} className="px-6 py-16 text-center text-sm text-muted-foreground">
                  No visible projects.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {showCreate && (
        <CreateProjectModal onClose={() => setShowCreate(false)} onCreate={onCreate} />
      )}
      {delConfirm && (
        <Confirm
          title={`Delete "${delConfirm.name}"`}
          body="This permanently deletes the project and all its versions and artifacts. This action cannot be undone."
          confirmLabel="Delete project"
          onConfirm={() => { onDelete(delConfirm.project_id); setDelConfirm(null); }}
          onCancel={() => setDelConfirm(null)}
        />
      )}
      {visConfirm && (
        <Confirm
          title="Change visibility"
          body={`Change "${visConfirm.name}" from ${visConfirm.visibility} to ${visConfirm.visibility === "public" ? "private" : "public"}?`}
          confirmLabel="Change visibility"
          onConfirm={() => { onToggleVis(visConfirm.project_id); setVisConfirm(null); }}
          onCancel={() => setVisConfirm(null)}
          danger={false}
        />
      )}
    </div>
  );
}

// ── Versions Tab ─────────────────────────────────────────────────────────────

function VersionsTab({ project, versions }: { project: Project; versions: VersionRecord[] }) {
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloaded,  setDownloaded]  = useState<string | null>(null);

  const pv = versions.filter(v => v.project_id === project.project_id);

  function dl(version: string) {
    setDownloading(version);
    setTimeout(() => {
      setDownloading(null);
      setDownloaded(version);
      setTimeout(() => setDownloaded(v => v === version ? null : v), 2500);
    }, 900);
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm min-w-[640px]">
        <thead className="border-b border-border">
          <tr>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">Version</th>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">Size</th>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">SHA-256</th>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">Published</th>
            <th className="text-right py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Download</th>
          </tr>
        </thead>
        <tbody>
          {pv.map((v, i) => (
            <tr key={v.version} className={`border-b border-border/50 ${i === 0 ? "bg-primary/[0.04]" : ""}`}>
              <td className="py-3 pr-6">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm text-foreground">{v.version}</span>
                  {i === 0 && <Badge variant="scope">latest</Badge>}
                </div>
              </td>
              <td className="py-3 pr-6 font-mono text-xs text-muted-foreground">{fmtSize(v.size)}</td>
              <td className="py-3 pr-6 font-mono text-xs text-muted-foreground">{v.sha256.slice(0, 16)}…</td>
              <td className="py-3 pr-6 text-xs text-muted-foreground whitespace-nowrap">{fmtDate(v.published_at)}</td>
              <td className="py-3 text-right">
                <Btn
                  size="sm"
                  variant={downloaded === v.version ? "outline" : "ghost"}
                  onClick={() => dl(v.version)}
                  disabled={downloading === v.version}
                >
                  {downloaded === v.version
                    ? <Check size={11} className="text-primary" />
                    : <Download size={11} />}
                  {downloading === v.version ? "Downloading…" : downloaded === v.version ? "Done" : ".tar.gz"}
                </Btn>
              </td>
            </tr>
          ))}
          {pv.length === 0 && (
            <tr>
              <td colSpan={5} className="py-12 text-center text-sm text-muted-foreground">
                No versions published yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

// ── Collaborators Tab ─────────────────────────────────────────────────────────

function AddCollaboratorModal({ onClose, onAdd }: {
  onClose: () => void;
  onAdd: (userId: number, role: "read" | "write" | "admin") => void;
}) {
  const [userId, setUserId] = useState("");
  const [role, setRole]     = useState<"read" | "write" | "admin">("read");
  const [error, setError]   = useState("");

  function handle() {
    const id = parseInt(userId, 10);
    if (!userId || isNaN(id) || id <= 0) { setError("Enter a valid numeric user ID."); return; }
    onAdd(id, role);
    onClose();
  }

  return (
    <Modal title="Add Collaborator" onClose={onClose}>
      <Field label="User ID (numeric)">
        <Inp
          type="number"
          placeholder="1042"
          value={userId}
          onChange={(e) => { setUserId(e.target.value); setError(""); }}
          autoFocus
        />
        <p className="text-xs text-muted-foreground mt-1.5 font-mono">
          No user directory API — enter the numeric user ID directly.
        </p>
      </Field>
      <Field label="Role">
        <Sel value={role} onChange={e => setRole(e.target.value as any)} className="w-full">
          <option value="read">read</option>
          <option value="write">write</option>
          <option value="admin">admin</option>
        </Sel>
      </Field>
      {error && <div className="mb-4"><Err msg={error} onDismiss={() => setError("")} /></div>}
      <div className="flex gap-2 justify-end mt-2">
        <Btn variant="outline" onClick={onClose}>Cancel</Btn>
        <Btn variant="primary" onClick={handle}><Users size={13} />Add</Btn>
      </div>
    </Modal>
  );
}

function CollaboratorsTab({ project, collaborators, user, onAdd, onChangeRole, onRemove }: {
  project: Project;
  collaborators: Collaborator[];
  user: User;
  onAdd: (uid: number, role: "read" | "write" | "admin") => void;
  onChangeRole: (uid: number, role: "read" | "write" | "admin") => void;
  onRemove: (uid: number) => void;
}) {
  const [showAdd,      setShowAdd]      = useState(false);
  const [removeTarget, setRemoveTarget] = useState<Collaborator | null>(null);
  const [error,        setError]        = useState("");

  const isOwner = project.owner === user.id;

  const roleVariant = (role: string): BV =>
    role === "admin" ? "role-admin" : role === "write" ? "role-write" : "role-read";

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <p className="text-xs text-muted-foreground font-mono">
          Owner (uid:{project.owner}) has implicit admin — not in list.
        </p>
        <Btn size="sm" variant="outline" onClick={() => isOwner ? setShowAdd(true) : setError("403 Forbidden — administration scope required.")}>
          <Plus size={12} />Add collaborator
        </Btn>
      </div>
      {error && <div className="mb-4"><Err msg={error} onDismiss={() => setError("")} /></div>}

      <table className="w-full text-sm">
        <thead className="border-b border-border">
          <tr>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">User ID</th>
            <th className="text-left py-3 pr-6 text-xs font-medium text-muted-foreground uppercase tracking-wide">Role</th>
            <th className="text-right py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Actions</th>
          </tr>
        </thead>
        <tbody>
          {collaborators.map(c => (
            <tr key={c.user_id} className="border-b border-border/50">
              <td className="py-3 pr-6 font-mono text-sm text-foreground">{c.user_id}</td>
              <td className="py-3 pr-6">
                <Sel
                  value={c.role}
                  onChange={e => {
                    if (!isOwner) { setError("403 Forbidden — administration scope required."); return; }
                    onChangeRole(c.user_id, e.target.value as any);
                  }}
                  className="text-xs py-1 px-2"
                >
                  <option value="read">read</option>
                  <option value="write">write</option>
                  <option value="admin">admin</option>
                </Sel>
              </td>
              <td className="py-3 text-right">
                <Btn size="sm" variant="ghost" onClick={() => isOwner ? setRemoveTarget(c) : setError("403 Forbidden — administration scope required.")}>
                  <Trash2 size={11} className="text-destructive" />
                </Btn>
              </td>
            </tr>
          ))}
          {collaborators.length === 0 && (
            <tr>
              <td colSpan={3} className="py-10 text-center text-sm text-muted-foreground">
                No collaborators added yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>

      {showAdd && (
        <AddCollaboratorModal
          onClose={() => setShowAdd(false)}
          onAdd={(uid, role) => {
            if (uid === project.owner) { setError(`403 Forbidden — owner's role cannot be managed.`); return; }
            onAdd(uid, role);
          }}
        />
      )}
      {removeTarget && (
        <Confirm
          title="Remove collaborator"
          body={`Remove user ${removeTarget.user_id} from this project? They will lose access immediately.`}
          confirmLabel="Remove"
          onConfirm={() => { onRemove(removeTarget.user_id); setRemoveTarget(null); }}
          onCancel={() => setRemoveTarget(null)}
        />
      )}
    </div>
  );
}

// ── Project Detail Page ───────────────────────────────────────────────────────

function ProjectDetailPage({ project, versions, collaborators, user, onBack, onDelete, onToggleVis, onAddCollab, onChangeRole, onRemoveCollab }: {
  project: Project;
  versions: VersionRecord[];
  collaborators: Collaborator[];
  user: User;
  onBack: () => void;
  onDelete: (id: number) => void;
  onToggleVis: (id: number) => void;
  onAddCollab: (uid: number, role: "read" | "write" | "admin") => void;
  onChangeRole: (uid: number, role: "read" | "write" | "admin") => void;
  onRemoveCollab: (uid: number) => void;
}) {
  const [tab,        setTab]        = useState<"versions" | "collaborators">("versions");
  const [delConfirm, setDelConfirm] = useState(false);
  const [visConfirm, setVisConfirm] = useState(false);
  const [error403,   setError403]   = useState("");

  const pv = versions.filter(v => v.project_id === project.project_id);
  const isOwner = project.owner === user.id;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border shrink-0">
        <div className="flex items-center gap-2 text-xs text-muted-foreground mb-3">
          <button onClick={onBack} className="flex items-center gap-1 hover:text-foreground transition">
            <ArrowLeft size={11} />Projects
          </button>
          <ChevronRight size={11} />
          <span className="text-foreground font-mono">{project.name}</span>
        </div>
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-3 min-w-0">
            <h1 className="font-mono text-base font-semibold text-foreground truncate">{project.name}</h1>
            <Badge variant={project.visibility}>
              {project.visibility === "public" ? <Globe size={9} /> : <Lock size={9} />}
              {project.visibility}
            </Badge>
            <span className="text-xs text-muted-foreground font-mono shrink-0">owner:{project.owner}</span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Btn size="sm" variant="ghost" onClick={() => isOwner ? setVisConfirm(true) : setError403("403 Forbidden — administration scope required.")}>
              {project.visibility === "public" ? <Lock size={11} /> : <Globe size={11} />}
              {project.visibility === "public" ? "Make private" : "Make public"}
            </Btn>
            <Btn size="sm" variant="ghost" onClick={() => isOwner ? setDelConfirm(true) : setError403("403 Forbidden — projects:delete scope required.")}>
              <Trash2 size={11} className="text-destructive" />Delete
            </Btn>
          </div>
        </div>
        <p className="text-xs text-muted-foreground font-mono mt-1">
          {pv.length} version{pv.length !== 1 ? "s" : ""}
        </p>
      </div>

      {error403 && (
        <div className="mx-6 mt-3">
          <Err msg={error403} onDismiss={() => setError403("")} />
        </div>
      )}

      {/* Tabs */}
      <div className="px-6 border-b border-border shrink-0">
        <div className="flex gap-0 -mb-px">
          {(["versions", "collaborators"] as const).map(t => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-4 py-2.5 text-sm border-b-2 transition-colors ${
                tab === t
                  ? "border-primary text-primary font-medium"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              {t === "versions" ? `Versions (${pv.length})` : "Collaborators"}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        {tab === "versions" && <VersionsTab project={project} versions={versions} />}
        {tab === "collaborators" && (
          <CollaboratorsTab
            project={project}
            collaborators={collaborators}
            user={user}
            onAdd={onAddCollab}
            onChangeRole={onChangeRole}
            onRemove={onRemoveCollab}
          />
        )}
      </div>

      {delConfirm && (
        <Confirm
          title={`Delete "${project.name}"`}
          body="This permanently deletes the project, all versions, and all artifacts. Cannot be undone."
          confirmLabel="Delete project"
          onConfirm={() => { onDelete(project.project_id); onBack(); }}
          onCancel={() => setDelConfirm(false)}
        />
      )}
      {visConfirm && (
        <Confirm
          title="Change visibility"
          body={`Change "${project.name}" to ${project.visibility === "public" ? "private" : "public"}?`}
          confirmLabel="Change"
          onConfirm={() => { onToggleVis(project.project_id); setVisConfirm(false); }}
          onCancel={() => setVisConfirm(false)}
          danger={false}
        />
      )}
    </div>
  );
}

// ── JWT Reveal Modal ──────────────────────────────────────────────────────────

function JwtReveal({ jwt, label, onClose }: { jwt: string; label: string; onClose: () => void }) {
  const [copied, setCopied] = useState(false);

  function handleCopy() {
    navigator.clipboard.writeText(jwt).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <Modal title={label} onClose={onClose}>
      <div className="flex items-start gap-2 bg-amber-950/40 border border-amber-800/50 text-amber-300 rounded-md p-3 mb-4 text-xs">
        <AlertTriangle size={13} className="shrink-0 mt-0.5" />
        <div>
          <div className="font-semibold mb-0.5">One-time display only</div>
          <div className="text-amber-300/70">This token will not be shown again. Copy it now and store it securely. The previous JWT is immediately invalidated.</div>
        </div>
      </div>
      <div className="bg-black/50 border border-border rounded-md p-3 mb-4">
        <code className="text-xs font-mono text-primary break-all leading-relaxed select-all">{jwt}</code>
      </div>
      <div className="flex gap-2 justify-end">
        <Btn variant="outline" onClick={handleCopy}>
          {copied ? <Check size={12} className="text-primary" /> : <Copy size={12} />}
          {copied ? "Copied!" : "Copy token"}
        </Btn>
        <Btn variant="primary" onClick={onClose}>Done</Btn>
      </div>
    </Modal>
  );
}

// ── Token Form Modal ──────────────────────────────────────────────────────────

function TokenFormModal({ title, init, projects, onClose, onSave, isEdit = false }: {
  title: string;
  init?: { name: string; projectScope: "All" | number[]; scopes: string[]; };
  projects: Project[];
  onClose: () => void;
  onSave: (data: { name: string; projectScope: "All" | number[]; scopes: string[]; expiresAt: string | null }) => void;
  isEdit?: boolean;
}) {
  const [name,       setName]       = useState(init?.name ?? "");
  const [scopeType,  setScopeType]  = useState<"All" | "Specified">(Array.isArray(init?.projectScope) ? "Specified" : "All");
  const [specified,  setSpecified]  = useState<number[]>(Array.isArray(init?.projectScope) ? init!.projectScope : []);
  const [scopes,     setScopes]     = useState<string[]>(init?.scopes ?? []);
  const [expiryType, setExpiryType] = useState<"none" | "date">("none");
  const [expiryDate, setExpiryDate] = useState(new Date(Date.now() + 365 * 864e5).toISOString().slice(0, 10));
  const [error,      setError]      = useState("");

  function toggleProject(id: number) {
    setSpecified(prev => prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]);
  }

  function toggleScope(s: string) {
    setScopes(prev => prev.includes(s) ? prev.filter(x => x !== s) : [...prev, s]);
  }

  function handleSave() {
    if (!name.trim()) { setError("Token name is required."); return; }
    if (scopeType === "Specified" && specified.length === 0) {
      setError("Select at least one project, or choose All."); return;
    }
    if (scopes.length === 0) { setError("Select at least one permission scope."); return; }
    if (expiryType === "date") {
      const d = new Date(expiryDate);
      if (isNaN(d.getTime())) { setError("Invalid expiry date."); return; }
      if (d > new Date(Date.now() + 366 * 864e5)) { setError("Expiry cannot exceed 1 year from today."); return; }
    }
    onSave({
      name: name.trim(),
      projectScope: scopeType === "All" ? "All" : specified,
      scopes,
      expiresAt: expiryType === "date" ? new Date(expiryDate).toISOString() : null,
    });
  }

  const maxDate = new Date(Date.now() + 365 * 864e5).toISOString().slice(0, 10);

  return (
    <Modal title={title} onClose={onClose} wide>
      <div className="grid grid-cols-2 gap-6">
        {/* Left column */}
        <div className="space-y-4">
          <Field label="Name">
            <Inp
              type="text"
              placeholder="CI/CD Token"
              value={name}
              onChange={e => { setName(e.target.value); setError(""); }}
              autoFocus
            />
          </Field>

          <div>
            <Lbl>Project Scope</Lbl>
            <div className="flex gap-2 mb-2">
              {(["All", "Specified"] as const).map(t => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setScopeType(t)}
                  className={`flex-1 py-1.5 text-xs rounded-md border transition ${
                    scopeType === t ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:border-white/20"
                  }`}
                >
                  {t === "All" ? "All projects" : "Specific projects"}
                </button>
              ))}
            </div>
            {scopeType === "Specified" && (
              <div className="space-y-0.5 max-h-36 overflow-y-auto border border-border rounded-md p-1">
                {projects.map(p => (
                  <label key={p.project_id} className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-white/5 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={specified.includes(p.project_id)}
                      onChange={() => toggleProject(p.project_id)}
                      className="accent-primary"
                    />
                    <span className="font-mono text-xs text-foreground flex-1">{p.name}</span>
                    <Badge variant={p.visibility}>{p.visibility}</Badge>
                  </label>
                ))}
              </div>
            )}
          </div>

          <div>
            <Lbl>Expiry</Lbl>
            <div className="flex gap-2 mb-2">
              {([{ v: "none", lbl: "No expiry" }, { v: "date", lbl: "Set date" }]).map(({ v, lbl }) => (
                <button
                  key={v}
                  type="button"
                  onClick={() => setExpiryType(v as any)}
                  className={`flex-1 py-1.5 text-xs rounded-md border transition ${
                    expiryType === v ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:border-white/20"
                  }`}
                >
                  {lbl}
                </button>
              ))}
            </div>
            {expiryType === "date" && (
              <Inp
                type="date"
                value={expiryDate}
                onChange={e => setExpiryDate(e.target.value)}
                min={new Date().toISOString().slice(0, 10)}
                max={maxDate}
              />
            )}
            {isEdit && (
              <p className="text-xs text-muted-foreground mt-1.5 font-mono leading-relaxed">
                expires_at: null = no change. Scope changes trigger re-sign + new JWT (once).
              </p>
            )}
          </div>
        </div>

        {/* Right column — scopes */}
        <div>
          <Lbl>Permission Scopes</Lbl>
          <div className="space-y-0.5 border border-border rounded-md p-1">
            {ALL_SCOPES.map(s => (
              <label key={s} className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-white/5 cursor-pointer">
                <input
                  type="checkbox"
                  checked={scopes.includes(s)}
                  onChange={() => toggleScope(s)}
                  className="accent-primary"
                />
                <span className="font-mono text-xs text-foreground">{s}</span>
              </label>
            ))}
          </div>
        </div>
      </div>

      {error && <div className="mt-4"><Err msg={error} onDismiss={() => setError("")} /></div>}

      {isEdit && (
        <div className="mt-4 p-3 bg-amber-950/30 border border-amber-800/40 rounded-md text-xs text-amber-400 font-mono leading-relaxed">
          Changing scopes or project scope re-signs the token. The old JWT is immediately invalidated and the new JWT is shown once.
        </div>
      )}

      <div className="flex gap-2 justify-end mt-5">
        <Btn variant="outline" onClick={onClose}>Cancel</Btn>
        <Btn variant="primary" onClick={handleSave}>
          {isEdit ? <><Edit3 size={12} />Save changes</> : <><Plus size={12} />Create token</>}
        </Btn>
      </div>
    </Modal>
  );
}

// ── Tokens Page ───────────────────────────────────────────────────────────────

function TokensPage({ tokens, projects, onCreate, onEdit, onRotate, onRevoke }: {
  tokens: TokenSummary[];
  projects: Project[];
  onCreate: (data: { name: string; projectScope: "All" | number[]; scopes: string[]; expiresAt: string | null }) => string;
  onEdit: (id: string, data: { name: string; projectScope: "All" | number[]; scopes: string[]; expiresAt: string | null }) => string | null;
  onRotate: (id: string) => string;
  onRevoke: (id: string) => void;
}) {
  const [showCreate,    setShowCreate]    = useState(false);
  const [editTarget,    setEditTarget]    = useState<TokenSummary | null>(null);
  const [rotateConfirm, setRotateConfirm] = useState<TokenSummary | null>(null);
  const [revokeConfirm, setRevokeConfirm] = useState<TokenSummary | null>(null);
  const [jwtReveal,     setJwtReveal]     = useState<{ jwt: string; label: string } | null>(null);

  function handleCreate(data: Parameters<typeof onCreate>[0]) {
    const jwt = onCreate(data);
    setShowCreate(false);
    setJwtReveal({ jwt, label: "Token created — copy your token now" });
  }

  function handleEdit(data: Parameters<typeof onEdit>[1]) {
    if (!editTarget) return;
    const jwt = onEdit(editTarget.token_id, data);
    setEditTarget(null);
    if (jwt) setJwtReveal({ jwt, label: "Token updated — new JWT issued" });
  }

  function handleRotate() {
    if (!rotateConfirm) return;
    const jwt = onRotate(rotateConfirm.token_id);
    setRotateConfirm(null);
    setJwtReveal({ jwt, label: `"${rotateConfirm.name}" rotated — new JWT` });
  }

  function handleRevoke() {
    if (!revokeConfirm) return;
    onRevoke(revokeConfirm.token_id);
    setRevokeConfirm(null);
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between shrink-0">
        <div className="flex items-center gap-3">
          <h1 className="text-sm font-semibold text-foreground">API Tokens</h1>
          <span className="font-mono text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">{tokens.length}</span>
        </div>
        <Btn variant="primary" size="sm" onClick={() => setShowCreate(true)}>
          <Plus size={13} />New Token
        </Btn>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm min-w-[700px]">
          <thead className="border-b border-border sticky top-0 bg-background z-10">
            <tr>
              <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Name</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Project Scope</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Permissions</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Created</th>
              <th className="text-right px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Actions</th>
            </tr>
          </thead>
          <tbody>
            {tokens.map(t => (
              <tr key={t.token_id} className="border-b border-border/50 hover:bg-white/[0.02] transition-colors">
                <td className="px-6 py-3">
                  <div className="text-sm font-medium text-foreground">{t.name}</div>
                  <div className="font-mono text-xs text-muted-foreground mt-0.5">{t.token_id}</div>
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground max-w-44">
                  {t.project_scope === "All"
                    ? <span className="text-foreground">All projects</span>
                    : <span className="font-mono">{scopeLabel(t.project_scope, projects)}</span>
                  }
                </td>
                <td className="px-4 py-3">
                  <div className="flex flex-wrap gap-1">
                    {t.scopes.map(s => <Badge key={s} variant="scope">{s}</Badge>)}
                  </div>
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground whitespace-nowrap font-mono">{fmtDate(t.created_at)}</td>
                <td className="px-6 py-3">
                  <div className="flex items-center gap-1 justify-end">
                    <Btn size="sm" variant="ghost" onClick={() => setEditTarget(t)}>
                      <Edit3 size={11} />Edit
                    </Btn>
                    <Btn size="sm" variant="ghost" onClick={() => setRotateConfirm(t)}>
                      <RotateCcw size={11} />Rotate
                    </Btn>
                    <Btn size="sm" variant="ghost" onClick={() => setRevokeConfirm(t)}>
                      <Trash2 size={11} className="text-destructive" />
                    </Btn>
                  </div>
                </td>
              </tr>
            ))}
            {tokens.length === 0 && (
              <tr>
                <td colSpan={5} className="px-6 py-16 text-center text-sm text-muted-foreground">
                  No API tokens. Create one to get started.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Modals */}
      {showCreate && (
        <TokenFormModal
          title="Create API Token"
          projects={projects}
          onClose={() => setShowCreate(false)}
          onSave={handleCreate}
        />
      )}
      {editTarget && (
        <TokenFormModal
          title={`Edit Token — ${editTarget.name}`}
          init={{
            name: editTarget.name,
            projectScope: editTarget.project_scope === "All" ? "All" : editTarget.project_scope.Specified,
            scopes: editTarget.scopes,
          }}
          projects={projects}
          onClose={() => setEditTarget(null)}
          onSave={handleEdit}
          isEdit
        />
      )}
      {rotateConfirm && (
        <Confirm
          title={`Rotate "${rotateConfirm.name}"`}
          body="Rotating issues a new JWT. The current token is immediately invalidated. The rotated token will have no expiry."
          confirmLabel="Rotate token"
          onConfirm={handleRotate}
          onCancel={() => setRotateConfirm(null)}
          danger={false}
        />
      )}
      {revokeConfirm && (
        <Confirm
          title={`Revoke "${revokeConfirm.name}"`}
          body="This immediately revokes the token. Any systems using it will lose access. This cannot be undone."
          confirmLabel="Revoke token"
          onConfirm={handleRevoke}
          onCancel={() => setRevokeConfirm(null)}
        />
      )}
      {jwtReveal && (
        <JwtReveal
          jwt={jwtReveal.jwt}
          label={jwtReveal.label}
          onClose={() => setJwtReveal(null)}
        />
      )}
    </div>
  );
}

// ── App Root ──────────────────────────────────────────────────────────────────

export default function App() {
  const [user,    setUser]    = useState<User | null>(null);
  const [view,    setView]    = useState<View>("projects");
  const [selProj, setSelProj] = useState<Project | null>(null);

  const [projects,       setProjects]       = useState<Project[]>(SEED_PROJECTS);
  const [tokens,         setTokens]         = useState<TokenSummary[]>(SEED_TOKENS);
  const [collaborators,  setCollaborators]  = useState<Record<number, Collaborator[]>>(SEED_COLLABORATORS);

  // ── Auth ──

  function handleLogin(u: User) {
    setUser(u);
    setView("projects");
  }

  function handleLogout() {
    setUser(null);
    setView("projects");
    setSelProj(null);
  }

  function handleNavigate(v: "projects" | "tokens") {
    setView(v);
    setSelProj(null);
  }

  // ── Projects ──

  function createProject(name: string, vis: "public" | "private") {
    const id = Math.max(...projects.map(p => p.project_id), 0) + 1;
    setProjects(prev => [...prev, { project_id: id, name, visibility: vis, owner: user!.id }]);
    setCollaborators(prev => ({ ...prev, [id]: [] }));
  }

  function deleteProject(id: number) {
    setProjects(prev => prev.filter(p => p.project_id !== id));
  }

  function toggleVisibility(id: number) {
    setProjects(prev =>
      prev.map(p => p.project_id === id
        ? { ...p, visibility: p.visibility === "public" ? "private" : "public" }
        : p
      )
    );
    if (selProj?.project_id === id) {
      setSelProj(prev => prev ? { ...prev, visibility: prev.visibility === "public" ? "private" : "public" } : null);
    }
  }

  // ── Tokens ──

  function createToken(data: { name: string; projectScope: "All" | number[]; scopes: string[]; expiresAt: string | null }): string {
    const t: TokenSummary = {
      token_id: "tok_" + uid(),
      name: data.name,
      project_scope: data.projectScope === "All" ? "All" : { Specified: data.projectScope },
      scopes: data.scopes,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    setTokens(prev => [...prev, t]);
    return mockJwt();
  }

  function editToken(id: string, data: { name: string; projectScope: "All" | number[]; scopes: string[]; expiresAt: string | null }): string | null {
    const existing = tokens.find(t => t.token_id === id);
    if (!existing) return null;

    const newScope = data.projectScope === "All" ? "All" : { Specified: data.projectScope };
    const scopeChanged =
      JSON.stringify(existing.scopes.slice().sort()) !== JSON.stringify(data.scopes.slice().sort()) ||
      JSON.stringify(existing.project_scope) !== JSON.stringify(newScope);

    setTokens(prev => prev.map(t =>
      t.token_id === id
        ? { ...t, name: data.name, project_scope: newScope, scopes: data.scopes, updated_at: new Date().toISOString() }
        : t
    ));
    return scopeChanged ? mockJwt() : null;
  }

  function rotateToken(id: string): string {
    setTokens(prev => prev.map(t => t.token_id === id ? { ...t, updated_at: new Date().toISOString() } : t));
    return mockJwt();
  }

  function revokeToken(id: string) {
    setTokens(prev => prev.filter(t => t.token_id !== id));
  }

  // ── Collaborators ──

  function addCollab(pid: number, uid2: number, role: "read" | "write" | "admin") {
    setCollaborators(prev => {
      const cur = prev[pid] ?? [];
      const exists = cur.find(c => c.user_id === uid2);
      return {
        ...prev,
        [pid]: exists ? cur.map(c => c.user_id === uid2 ? { ...c, role } : c) : [...cur, { user_id: uid2, role }],
      };
    });
  }

  function changeRole(pid: number, uid2: number, role: "read" | "write" | "admin") {
    setCollaborators(prev => ({
      ...prev,
      [pid]: (prev[pid] ?? []).map(c => c.user_id === uid2 ? { ...c, role } : c),
    }));
  }

  function removeCollab(pid: number, uid2: number) {
    setCollaborators(prev => ({
      ...prev,
      [pid]: (prev[pid] ?? []).filter(c => c.user_id !== uid2),
    }));
  }

  // ── Render ──

  if (!user) return <LoginPage onLogin={handleLogin} />;

  const liveProject = selProj ? projects.find(p => p.project_id === selProj.project_id) ?? selProj : null;

  return (
    <div className="flex h-screen bg-background overflow-hidden font-sans">
      <Sidebar user={user} view={view} onNavigate={handleNavigate} onLogout={handleLogout} />
      <main className="flex-1 overflow-hidden">
        {view === "projects" && (
          <ProjectsPage
            projects={projects}
            user={user}
            onSelect={p => { setSelProj(p); setView("project-detail"); }}
            onCreate={createProject}
            onDelete={deleteProject}
            onToggleVis={toggleVisibility}
          />
        )}
        {view === "project-detail" && liveProject && (
          <ProjectDetailPage
            project={liveProject}
            versions={SEED_VERSIONS}
            collaborators={collaborators[liveProject.project_id] ?? []}
            user={user}
            onBack={() => handleNavigate("projects")}
            onDelete={id => { deleteProject(id); handleNavigate("projects"); }}
            onToggleVis={toggleVisibility}
            onAddCollab={(uid2, role) => addCollab(liveProject.project_id, uid2, role)}
            onChangeRole={(uid2, role) => changeRole(liveProject.project_id, uid2, role)}
            onRemoveCollab={uid2 => removeCollab(liveProject.project_id, uid2)}
          />
        )}
        {view === "tokens" && (
          <TokensPage
            tokens={tokens}
            projects={projects}
            onCreate={createToken}
            onEdit={editToken}
            onRotate={rotateToken}
            onRevoke={revokeToken}
          />
        )}
      </main>
    </div>
  );
}
