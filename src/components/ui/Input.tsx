import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";
import { cn } from "./cn";

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Rendered inside the field, before the text (e.g. a search glyph). */
  leading?: ReactNode;
  /** Rendered inside the field, after the text (e.g. a `Kbd` hint). */
  trailing?: ReactNode;
  size?: "md" | "lg";
  /** Marks the field invalid and wires `aria-invalid`. */
  invalid?: boolean;
  /** Class applied to the outer shell rather than the `<input>`. */
  wrapperClassName?: string;
}

/**
 * Text field primitive (design plan §5).
 *
 * The visible border lives on a wrapper so leading/trailing adornments sit
 * inside the control, and focus is expressed with `focus-within` on that
 * wrapper. Dimensions stay fixed across states so focusing a field never
 * shifts the surrounding layout.
 */
export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  {
    leading,
    trailing,
    size = "md",
    invalid = false,
    className = "",
    wrapperClassName = "",
    ...rest
  },
  ref,
) {
  return (
    <div
      className={cn(
        "flex items-center gap-1.5 rounded-md border bg-surface-inset px-2.5",
        "transition-colors duration-fast ease-out",
        "focus-within:border-ring/70 focus-within:bg-card",
        size === "lg" ? "h-control-lg" : "h-control-md",
        invalid ? "border-destructive/60" : "border-input",
        wrapperClassName,
      )}
    >
      {leading != null && (
        <span className="flex shrink-0 items-center text-muted-foreground">{leading}</span>
      )}
      <input
        ref={ref}
        aria-invalid={invalid || undefined}
        className={cn(
          "min-w-0 flex-1 bg-transparent text-label text-foreground outline-none",
          "placeholder:text-muted-foreground-subtle",
          "disabled:cursor-not-allowed disabled:opacity-50",
          className,
        )}
        {...rest}
      />
      {trailing != null && <span className="flex shrink-0 items-center">{trailing}</span>}
    </div>
  );
});
