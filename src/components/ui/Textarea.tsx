import { forwardRef, type TextareaHTMLAttributes } from "react";
import { cn } from "./cn";

export interface TextareaProps
  extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  /** Marks the field invalid and wires `aria-invalid`. */
  invalid?: boolean;
  /** Renders the value in the mono stack — for JSON and other code payloads. */
  mono?: boolean;
}

/**
 * Multi-line text field (design plan §5).
 *
 * Unlike {@link Input} the border sits directly on the element: a textarea has
 * no adornments to host, and wrapping it would break the native resize grip.
 */
export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  function Textarea({ invalid = false, mono = false, className = "", ...rest }, ref) {
    return (
      <textarea
        ref={ref}
        aria-invalid={invalid || undefined}
        className={cn(
          "w-full rounded-md border bg-surface-inset px-2.5 py-1.5",
          "text-label text-foreground outline-none",
          "transition-colors duration-fast ease-out",
          "focus:border-ring/70 focus:bg-card",
          "placeholder:text-muted-foreground-subtle",
          "disabled:cursor-not-allowed disabled:opacity-50",
          invalid ? "border-destructive/60" : "border-input",
          mono && "font-mono",
          className,
        )}
        {...rest}
      />
    );
  },
);
