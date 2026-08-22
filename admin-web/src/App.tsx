import { useSyncExternalStore, type ReactElement } from "react";
import {
  BrowserRouter,
  Link,
  Navigate,
  NavLink,
  Route,
  Routes,
  useLocation,
  useNavigate,
  useParams,
} from "react-router-dom";
import { apiClient, sessionStore } from "./api/session";
import { Icon } from "./components/icons";
import { LanguageSwitcher } from "./components/ui";
import { ProtectedRoute } from "./components/ProtectedRoute";
import { LanguageProvider, useT } from "./i18n";
import { LoginPage } from "./pages/LoginPage";
import { ProjectDetailPage } from "./pages/ProjectDetailPage";
import { ProjectsPage } from "./pages/ProjectsPage";
import { TokensPage } from "./pages/TokensPage";

export function App(): ReactElement {
  return (
    <LanguageProvider>
      <BrowserRouter>
        <Shell />
      </BrowserRouter>
    </LanguageProvider>
  );
}

function Sidebar(): ReactElement | null {
  useSyncExternalStore(sessionStore.subscribe, sessionStore.getSnapshot);
  const navigate = useNavigate();
  const location = useLocation();
  const t = useT();
  const user = sessionStore.currentUser;
  if (!user) {
    return null;
  }
  const navItems = [
    { to: "/projects", label: t("nav.projects"), icon: "package" as const },
    { to: "/tokens", label: t("nav.tokens"), icon: "key" as const },
  ];
  const projectsActive = location.pathname.startsWith("/projects");
  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <Icon name="terminal" size={15} className="sidebar-brand-icon" />
        <span className="sidebar-brand-name">filehub</span>
      </div>
      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              isActive || (item.to === "/projects" && projectsActive)
                ? "sidebar-link active"
                : "sidebar-link"
            }
          >
            <Icon name={item.icon} size={14} />
            {item.label}
          </NavLink>
        ))}
      </nav>
      <div className="sidebar-footer">
        <div className="sidebar-user">
          <span className="avatar">{user.name[0]?.toUpperCase() ?? "?"}</span>
          <span className="sidebar-user-meta">
            <span className="sidebar-user-name">{user.name}</span>
            <span className="sidebar-user-id">{t("nav.userId", { id: user.id })}</span>
          </span>
        </div>
        <LanguageSwitcher />
        <button
          type="button"
          className="sidebar-signout"
          onClick={() => {
            sessionStore.logout();
            navigate("/login", { replace: true });
          }}
        >
          <Icon name="logout" size={12} />
          {t("nav.signout")}
        </button>
      </div>
    </aside>
  );
}

function Shell(): ReactElement {
  useSyncExternalStore(sessionStore.subscribe, sessionStore.getSnapshot);
  const authenticated = sessionStore.state === "authenticated";
  return (
    <div className="app-shell">
      {authenticated && <Sidebar />}
      <main className="app-main">
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <Navigate to="/projects" replace />
              </ProtectedRoute>
            }
          />
          <Route
            path="/projects"
            element={
              <ProtectedRoute>
                <ProjectsPage client={apiClient} />
              </ProtectedRoute>
            }
          />
          <Route
            path="/projects/:id"
            element={
              <ProtectedRoute>
                <ProjectDetailPage client={apiClient} />
              </ProtectedRoute>
            }
          />
          <Route
            path="/projects/:id/members"
            element={
              <ProtectedRoute>
                <MemberRedirect />
              </ProtectedRoute>
            }
          />
          <Route
            path="/tokens"
            element={
              <ProtectedRoute>
                <TokensPage client={apiClient} />
              </ProtectedRoute>
            }
          />
          <Route path="*" element={<NotFoundPage />} />
        </Routes>
      </main>
    </div>
  );
}

function MemberRedirect(): ReactElement {
  const { id } = useParams<{ id: string }>();
  if (id) {
    return <Navigate to={`/projects/${encodeURIComponent(id)}?tab=collaborators`} replace />;
  }
  return <Navigate to="/projects" replace />;
}

function NotFoundPage(): ReactElement {
  const t = useT();
  useSyncExternalStore(sessionStore.subscribe, sessionStore.getSnapshot);
  return (
    <div className="empty-state">
      <p className="hint">
        {t("notFound.title")}{" "}
        <Link to={sessionStore.state === "authenticated" ? "/projects" : "/login"}>
          {t("notFound.back")}
        </Link>
      </p>
    </div>
  );
}
