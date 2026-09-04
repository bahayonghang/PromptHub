/**
 * Shared UI primitives.
 *
 * These sit between the design tokens in `src/styles/globals.css` and the
 * feature components. Feature code should compose these rather than
 * re-deriving control heights, radii, focus rings, or disabled opacity from
 * raw Tailwind strings (design plan §2).
 */
export { cn } from "./cn";
export { Button, type ButtonProps, type ButtonVariant, type ButtonSize } from "./Button";
export {
  IconButton,
  type IconButtonProps,
  type IconButtonVariant,
  type IconButtonSize,
} from "./IconButton";
export { Input, type InputProps } from "./Input";
export { Select, type SelectProps, type SelectOption } from "./Select";
export { Tag, type TagProps } from "./Tag";
export { tagSlot, tagClasses, TAG_SLOT_COUNT, type TagSlot } from "./tagColor";
export { Kbd } from "./Kbd";
export { Panel, type PanelProps } from "./Panel";
export { UsageBar, type UsageBarProps } from "./UsageBar";
export { EmptyState, type EmptyStateProps } from "./EmptyState";
export { ConfirmDialog, type ConfirmDialogProps } from "./ConfirmDialog";
export { Modal, type ModalProps } from "./Modal";
