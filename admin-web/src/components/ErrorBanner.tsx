import type { ReactElement } from "react";

export function ErrorBanner({ message }: { message: string }): ReactElement {
  if (!message) {
    return <></>;
  }
  return <div className="error-banner" role="alert">{message}</div>;
}
