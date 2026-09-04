/** Number of slots in the global tag palette (`--tag-1` … `--tag-8`). */
export const TAG_SLOT_COUNT = 8;

export type TagSlot = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;

/**
 * Maps a tag name to a palette slot.
 *
 * Tags are user-created free text, so colours cannot be authored by hand. A
 * stable hash gives every tag the same hue everywhere it appears — sidebar
 * cloud, filter bar, list row, card, detail panel — which is what makes the
 * colour useful for scanning rather than decorative.
 *
 * The function must stay pure and deterministic: the same name always yields
 * the same slot, across sessions and across machines. FNV-1a is used because it
 * is short, has good avalanche behaviour on short ASCII/CJK strings, and needs
 * no dependency.
 */
export function tagSlot(name: string): TagSlot {
  // FNV-1a (32-bit). `Math.imul` keeps the multiply in 32-bit space.
  let hash = 0x811c9dc5;
  for (let i = 0; i < name.length; i += 1) {
    hash ^= name.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  // `>>> 0` reinterprets the value as unsigned before taking the modulus.
  return (((hash >>> 0) % TAG_SLOT_COUNT) + 1) as TagSlot;
}

/**
 * Tailwind class fragments per slot. Written as complete literal class names so
 * Tailwind's content scanner can see them; a template string such as
 * `text-tag-${slot}` would be purged from the production build.
 */
const SLOT_CLASSES: Record<TagSlot, string> = {
  1: "text-tag-1 bg-tag-1/15 border-tag-1/25",
  2: "text-tag-2 bg-tag-2/15 border-tag-2/25",
  3: "text-tag-3 bg-tag-3/15 border-tag-3/25",
  4: "text-tag-4 bg-tag-4/15 border-tag-4/25",
  5: "text-tag-5 bg-tag-5/15 border-tag-5/25",
  6: "text-tag-6 bg-tag-6/15 border-tag-6/25",
  7: "text-tag-7 bg-tag-7/15 border-tag-7/25",
  8: "text-tag-8 bg-tag-8/15 border-tag-8/25",
};

/** Selected/pressed variant: same hue, more fill and a firmer border. */
const SLOT_CLASSES_ACTIVE: Record<TagSlot, string> = {
  1: "text-tag-1 bg-tag-1/25 border-tag-1/55",
  2: "text-tag-2 bg-tag-2/25 border-tag-2/55",
  3: "text-tag-3 bg-tag-3/25 border-tag-3/55",
  4: "text-tag-4 bg-tag-4/25 border-tag-4/55",
  5: "text-tag-5 bg-tag-5/25 border-tag-5/55",
  6: "text-tag-6 bg-tag-6/25 border-tag-6/55",
  7: "text-tag-7 bg-tag-7/25 border-tag-7/55",
  8: "text-tag-8 bg-tag-8/25 border-tag-8/55",
};

export function tagClasses(name: string, active = false): string {
  const slot = tagSlot(name);
  return active ? SLOT_CLASSES_ACTIVE[slot] : SLOT_CLASSES[slot];
}
