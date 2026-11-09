import React, { useEffect } from "react";
import { X } from "lucide-react";

type ButtonVariant = "default" | "primary" | "ghost" | "danger";

export function Button({
  variant = "default",
  size = "md",
  className = "",
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: "sm" | "md";
}) {
  const cls = [
    "btn",
    variant === "primary" ? "btn-primary" : "",
    variant === "ghost" ? "btn-ghost" : "",
    variant === "danger" ? "btn-danger" : "",
    size === "sm" ? "btn-sm" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button type="button" className={cls} {...rest}>
      {children}
    </button>
  );
}

export function IconButton({
  active,
  className = "",
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean }) {
  return (
    <button
      type="button"
      className={`icon-btn ${active ? "active" : ""} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}

export function Field({
  label,
  hint,
  children,
  className = "",
}: {
  label?: string;
  hint?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`mb-3 ${className}`}>
      {label ? <label className="label">{label}</label> : null}
      {children}
      {hint ? <p className="mt-1 text-[11px] leading-snug text-[var(--muted-2)]">{hint}</p> : null}
    </div>
  );
}

export function Input(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={`input ${props.className ?? ""}`} />;
}

export function Textarea(props: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} className={`textarea ${props.className ?? ""}`} />;
}

export function Select({
  options,
  placeholder,
  ...rest
}: React.SelectHTMLAttributes<HTMLSelectElement> & {
  options: { value: string; label: string }[];
  placeholder?: string;
}) {
  return (
    <select {...rest} className={`select ${rest.className ?? ""}`}>
      {placeholder ? <option value="">{placeholder}</option> : null}
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
}) {
  return (
    <label className="mb-3 flex cursor-pointer select-none items-center gap-2 text-[13px] text-[var(--text)]">
      <span
        className={`flex h-4 w-4 items-center justify-center rounded border transition-colors ${
          checked
            ? "border-[var(--accent)] bg-[var(--accent)]"
            : "border-[var(--border)] bg-[var(--bg-soft)]"
        }`}
        onClick={() => onChange(!checked)}
      >
        {checked ? (
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M1.5 5.2 4 7.6 8.6 2.4" stroke="white" strokeWidth="1.8" strokeLinecap="round" />
          </svg>
        ) : null}
      </span>
      <span onClick={() => onChange(!checked)}>{label}</span>
    </label>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  className = "",
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  className?: string;
}) {
  return (
    <div className={`inline-flex rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] p-0.5 ${className}`}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => onChange(option.value)}
          className={`rounded-md px-2.5 py-1 text-[12px] font-medium transition-colors ${
            value === option.value
              ? "bg-[var(--panel-3)] text-[var(--text)]"
              : "text-[var(--muted)] hover:text-[var(--text)]"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function Modal({
  open,
  onClose,
  title,
  subtitle,
  width = 720,
  children,
  footer,
}: {
  open: boolean;
  onClose: () => void;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  width?: number;
  children: React.ReactNode;
  footer?: React.ReactNode;
}) {
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-6"
      style={{ background: "rgba(5,5,10,0.62)", backdropFilter: "blur(3px)" }}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className="animate-in flex max-h-[88vh] w-full flex-col overflow-hidden rounded-2xl border border-[var(--border)] bg-[var(--panel)]"
        style={{ maxWidth: width, boxShadow: "var(--shadow)" }}
      >
        <div className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-5 py-4">
          <div>
            <h2 className="text-[15px] font-semibold text-[var(--text)]">{title}</h2>
            {subtitle ? (
              <p className="mt-0.5 max-w-[560px] text-[12px] leading-relaxed text-[var(--muted)]">{subtitle}</p>
            ) : null}
          </div>
          <IconButton onClick={onClose} aria-label="Close">
            <X size={16} />
          </IconButton>
        </div>
        <div className="scroll flex-1 px-5 py-4">{children}</div>
        {footer ? (
          <div className="flex items-center justify-end gap-2 border-t border-[var(--border)] bg-[var(--panel-2)] px-5 py-3">
            {footer}
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function Empty({
  icon,
  title,
  hint,
  action,
}: {
  icon?: React.ReactNode;
  title: string;
  hint?: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 px-8 py-14 text-center">
      {icon ? <div className="text-[var(--muted-2)] opacity-70">{icon}</div> : null}
      <div className="text-[14px] font-medium text-[var(--text)]">{title}</div>
      {hint ? <div className="max-w-[360px] text-[12px] leading-relaxed text-[var(--muted)]">{hint}</div> : null}
      {action}
    </div>
  );
}

export function Spinner({ size = 14 }: { size?: number }) {
  return (
    <span
      className="inline-block animate-spin rounded-full border-2 border-[var(--muted-2)] border-t-transparent"
      style={{ width: size, height: size }}
    />
  );
}

export function SectionHeader({
  title,
  right,
  className = "",
}: {
  title: React.ReactNode;
  right?: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`section-title ${className}`}>
      <span>{title}</span>
      {right}
    </div>
  );
}
