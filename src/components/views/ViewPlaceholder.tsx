import type { LucideIcon } from "lucide-react";

interface ViewPlaceholderProps {
  /** Icon representing the view (Req 22.4). */
  icon: LucideIcon;
  /** Already-translated heading for the view. */
  title: string;
  /** Already-translated supporting description. */
  description: string;
}

/**
 * A neutral placeholder for a content region whose full implementation lands in
 * a later task (22.2–22.5). It renders the view's icon, title, and a short
 * description, all driven by the active theme's design tokens (Req 22.3) and
 * laid out to stay centered without clipping or overlap (Req 23.6).
 */
export function ViewPlaceholder({ icon: Icon, title, description }: ViewPlaceholderProps) {
  return (
    <section className="flex h-full w-full flex-col items-center justify-center gap-3 p-8 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-accent text-accent-foreground">
        <Icon className="h-7 w-7" aria-hidden="true" />
      </span>
      <h2 className="text-display font-semibold text-foreground">{title}</h2>
      <p className="max-w-md text-body text-muted-foreground">{description}</p>
    </section>
  );
}
