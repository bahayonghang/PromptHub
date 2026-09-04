import { forwardRef, type TextareaHTMLAttributes } from "react";
import { cn } from "./cn";

export interface TextareaProps
  extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "size"> {
  /** Marks the field invalid and wires `aria-invalid`. */
  invalid?: boolean;
  /** Renders the value in the mono stack — for JSON and other code payloads. */
  mono?: boolean;
  /**
   * `md` is the dense workbench field; `lg` matches the prompt editor's body
   * text, where the textarea *is* the primary content rather than a side input.
   */
  size?: "md" | "lg";
  /** Allows the user to drag-resize vertically. */
  resizable?: boolean;
}

/**
 * Multi-line text field (design plan §5).
 *
 * Unlike {@link Input} the border sits directly on the element: a textarea has
 * no adornments to host, and wrapping it would break the native resize grip.
 */
export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  function Textarea(
    {
      invalid = false,
      mono = false,
      size = "md",
      resizable = false,
      className = "",
      ...rest
    },
    ref,
  ) {
    return (
      <textarea
        ref={ref}
        aria-invalid={invalid || undefined}
        className={cn(
          "w-full rounded-md border bg-surface-inset",
          size === "lg" ? "px-3 py-2 text-body" : "px-2.5 py-1.5 text-label",
          resizable ? "resize-y" : "resize-none",
          "text-foreground outline-none",
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
