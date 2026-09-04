import { cn } from "./cn";

export interface SwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  /** Accessible name. Omit only when the switch is wired via `aria-labelledby`. */
  label?: string;
  labelledBy?: string;
  describedBy?: string;
  disabled?: boolean;
  className?: string;
}

/**
 * Toggle switch (design plan §5).
 *
 * The knob is `bg-card` rather than a hardcoded white so it stays a *surface*
 * in both theme families — on the dark themes a pure-white knob glares against
 * the muted Signal Blue track.
 */
export function Switch({
  checked,
  onChange,
  label,
  labelledBy,
  describedBy,
  disabled = false,
  className = "",
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative h-6 w-11 shrink-0 rounded-full border",
        "transition-colors duration-fast ease-out",
        "disabled:cursor-not-allowed disabled:opacity-50",
        checked ? "border-primary bg-primary" : "border-input bg-input",
        className,
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "absolute left-0.5 top-0.5 h-[18px] w-[18px] rounded-full bg-card shadow-sm",
          "transition-transform duration-base ease-spring",
          checked ? "translate-x-5" : "translate-x-0",
        )}
      />
    </button>
  );
}
