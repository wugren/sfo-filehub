// 原型组件基元：按钮、徽标、输入、Modal、Confirm、错误横幅、空态与语言切换。
import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
} from "react";
import { useLanguage } from "../i18n";
import { Icon, type IconName } from "./icons";

// ── Badge ──────────────────────────────────────────────────────────────────

export type BadgeVariant =
  | "neutral"
  | "public"
  | "private"
  | "scope"
  | "danger"
  | "role-read"
  | "role-write"
  | "role-admin";

export function Badge({
  children,
  variant = "neutral",
  icon,
}: {
  children: ReactNode;
  variant?: BadgeVariant;
  icon?: IconName;
}): ReactNode {
  return (
    <span className={`badge badge-${variant}`}>
      {icon && <Icon name={icon} size={9} />}
      {children}
    </span>
  );
}

// ── Button ─────────────────────────────────────────────────────────────────

export type BtnVariant = "primary" | "ghost" | "danger" | "outline";
export type BtnSize = "sm" | "md";

export function Btn({
  children,
  variant = "primary",
  size = "md",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: BtnVariant;
  size?: BtnSize;
}): ReactNode {
  const classes = ["btn", `btn-${variant}`, `btn-${size}`, className].filter(Boolean).join(" ");
  return (
    <button type="button" {...props} className={classes}>
      {children}
    </button>
  );
}

// ── 表单基元 ───────────────────────────────────────────────────────────────

export function Inp(props: InputHTMLAttributes<HTMLInputElement>): ReactNode {
  return <input {...props} className={`inp ${props.className ?? ""}`.trim()} />;
}

export function Sel({
  children,
  className = "",
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>): ReactNode {
  return (
    <select {...props} className={`sel ${className}`.trim()}>
      {children}
    </select>
  );
}

export function Field({
  label,
  children,
  hint,
  htmlFor,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
  htmlFor?: string;
}): ReactNode {
  return (
    <div className="field">
      {htmlFor ? <label className="field-label" htmlFor={htmlFor}>{label}</label> : <span className="field-label">{label}</span>}
      {children}
      {hint && <p className="field-hint">{hint}</p>}
    </div>
  );
}

// ── Modal / Confirm ────────────────────────────────────────────────────────

export function Modal({
  title,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}): ReactNode {
  return (
    <div
      className="modal-overlay"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        className={`modal-card ${wide ? "modal-wide" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="modal-head">
          <h2>{title}</h2>
          <button type="button" className="modal-close" aria-label="close" onClick={onClose}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="modal-body">{children}</div>
      </div>
    </div>
  );
}

export function Confirm({
  title,
  body,
  confirmLabel,
  onConfirm,
  onCancel,
  danger = true,
}: {
  title: string;
  body: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  danger?: boolean;
}): ReactNode {
  const { t } = useLanguage();
  return (
    <Modal title={title} onClose={onCancel}>
      <p className="confirm-body">{body}</p>
      <div className="modal-actions">
        <Btn variant="outline" onClick={onCancel}>{t("common.cancel")}</Btn>
        <Btn variant={danger ? "danger" : "primary"} onClick={onConfirm}>{confirmLabel}</Btn>
      </div>
    </Modal>
  );
}

// ── 错误横幅 / 空态 ────────────────────────────────────────────────────────

export function ErrorBanner({
  message,
  onDismiss,
}: {
  message: string;
  onDismiss?: () => void;
}): ReactNode {
  if (!message) {
    return <></>;
  }
  return (
    <div className="error-banner" role="alert">
      <Icon name="alert" size={13} className="error-icon" />
      <span className="error-text">{message}</span>
      {onDismiss && (
        <button type="button" className="error-dismiss" onClick={onDismiss} aria-label="dismiss">
          <Icon name="x" size={12} />
        </button>
      )}
    </div>
  );
}

export function EmptyState({ children }: { children: ReactNode }): ReactNode {
  return <div className="empty-state">{children}</div>;
}

// ── 语言切换 ───────────────────────────────────────────────────────────────

export function LanguageSwitcher({ compact = false }: { compact?: boolean }): ReactNode {
  const { lang, setLang } = useLanguage();
  return (
    <div className={`lang-switch ${compact ? "lang-switch-compact" : ""}`} aria-label="language">
      <button
        type="button"
        className={lang === "zh" ? "active" : undefined}
        onClick={() => setLang("zh")}
      >
        中文
      </button>
      <button
        type="button"
        className={lang === "en" ? "active" : undefined}
        onClick={() => setLang("en")}
      >
        English
      </button>
    </div>
  );
}

export type { IconName };
