import { forwardRef, type SelectHTMLAttributes, type ReactNode } from "react";
import { ChevronDownIcon } from "lucide-react";
import { cn } from "./cn";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  /**
   * Convenience form for a flat list. Omit it and pass `children` instead when
   * the call site needs `<optgroup>`, a placeholder entry, or per-option
   * attributes.
   */
  options?: readonly SelectOption[];
  size?: "md" | "lg";
  /** Rendered before the value (e.g. a sort glyph). */
  leading?: ReactNode;
  block?: boolean;
  wrapperClassName?: string;
}

/**
 * Themed select (design plan §5).
 *
 * A bare `<select>` renders with OS-native chrome, which on a dark theme often
 * paints a light popup and visibly breaks the window's material. Rather than
 * hand-rolling a listbox — which would mean reimplementing typeahead, keyboard
 * semantics, and the platform popup layer — the native element is kept for
 * behaviour and accessibility, made transparent, and overlaid on a themed
 * shell. The dropdown list itself is still OS-drawn; `color-scheme` on the root
 * keeps it in the right mode.
 */
export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  {
    options,
    children,
    size = "md",
    leading,
    block = false,
    className = "",
    wrapperClassName = "",
    disabled,
    ...rest
  },
  ref,
) {
  return (
    <div
      className={cn(
        "relative inline-flex items-center gap-1.5 rounded-md border border-input bg-card pl-2.5 pr-2",
        "shadow-hairline transition-colors duration-fast ease-out",
        "focus-within:border-ring/70",
        !disabled && "hover:border-border-strong hover:bg-state-hover",
        disabled && "opacity-50",
        size === "lg" ? "h-control-lg" : "h-control-md",
        block ? "flex w-full" : "w-auto",
        wrapperClassName,
      )}
    >
      {leading != null && (
        <span className="flex shrink-0 items-center text-muted-foreground">{leading}</span>
      )}
      <select
        ref={ref}
        disabled={disabled}
        className={cn(
          // `appearance-none` removes the native arrow; the element stays in the
          // a11y tree and keeps native keyboard behaviour.
          "min-w-0 flex-1 appearance-none bg-transparent pr-4 text-label text-foreground",
          "outline-none",
          "disabled:cursor-not-allowed",
          className,
        )}
        {...rest}
      >
        {options
          ? options.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))
          : children}
      </select>
      <ChevronDownIcon
        aria-hidden="true"
        className="pointer-events-none absolute right-2 h-3.5 w-3.5 text-muted-foreground-subtle"
      />
    </div>
  );
});
