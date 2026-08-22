import type { ReactElement } from "react";
import { useSyncExternalStore } from "react";
import { Navigate, useLocation } from "react-router-dom";
import { sessionStore } from "../api/session";

export function ProtectedRoute({ children }: { children: ReactElement }): ReactElement {
  useSyncExternalStore(sessionStore.subscribe, sessionStore.getSnapshot);
  const location = useLocation();
  if (sessionStore.state !== "authenticated") {
    const next = encodeURIComponent(location.pathname + location.search);
    return <Navigate to={`/login?next=${next}`} replace />;
  }
  return children;
}
