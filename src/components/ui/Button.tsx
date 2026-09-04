import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { cn } from "./cn";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Renders before the label; use a 14px Lucide icon. */
  leadingIcon?: ReactNode;
  /** Renders after the label (e.g. a chevron or a `Kbd`). */
  trailingIcon?: ReactNode;
  /** Stretches the control to the width of its container. */
  block?: boolean;
}

/**
 * The shared button primitive (design plan §5).
 *
 * Replaces the ad-hoc `<button className="rounded-md bg-primary px-4 py-2 ...">`
 * strings scattered across the app, which had drifted into five different
 * `disabled:opacity-*` values, four control heights, and inconsistent focus
 * rings. Every visual decision here comes from a design token:
 *
 * - height from `--control-sm|md|lg`
 * - radius from `--radius` (never a bare `rounded`)
 * - the resting edge highlight from `--hairline`
 * - motion from `--dur-fast` / `--ease-out`
 *
 * A focus ring is always present; `type` defaults to `"button"` so a button
 * inside a form never submits it by accident.
 */
const BASE =
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md font-medium " +
  "whitespace-nowrap transition-colors duration-fast ease-out " +
  " " +
  " " +
  "disabled:pointer-events-none disabled:opacity-50";

const VARIANTS: Record<ButtonVariant, string> = {
  primary: "bg-primary text-primary-foreground shadow-hairline hover:bg-primary/90",
  secondary:
    "border border-border bg-card text-foreground shadow-hairline " +
    "hover:bg-state-hover hover:border-border-strong",
  ghost: "text-muted-foreground hover:bg-state-hover hover:text-foreground",
  danger:
    "border border-destructive/40 text-destructive hover:bg-destructive/10 " +
    "hover:border-destructive/60",
};

const SIZES: Record<ButtonSize, string> = {
  sm: "h-control-sm px-2.5 text-label",
  md: "h-control-md px-3 text-label",
  lg: "h-control-lg px-3.5 text-body",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "secondary",
    size = "md",
    leadingIcon,
    trailingIcon,
    block = false,
    className = "",
    type = "button",
    children,
    ...rest
  },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(BASE, VARIANTS[variant], SIZES[size], block && "w-full", className)}
      {...rest}
    >
      {leadingIcon}
      {children}
      {trailingIcon}
    </button>
  );
});
