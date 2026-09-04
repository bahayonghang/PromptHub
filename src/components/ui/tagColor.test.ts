import { describe, expect, it } from "vitest";
import fc from "fast-check";
import { TAG_SLOT_COUNT, tagClasses, tagSlot } from "./tagColor";

/**
 * The global tag palette is only useful if a tag keeps the same hue everywhere
 * it is rendered. That guarantee rests on `tagSlot` being a pure, deterministic
 * function of the tag name (design plan §10.1).
 */
describe("tagSlot", () => {
  it("is deterministic for a given name", () => {
    fc.assert(
      fc.property(fc.string(), (name) => {
        expect(tagSlot(name)).toBe(tagSlot(name));
      }),
      { numRuns: 200 },
    );
  });

  it("always lands inside the declared palette range", () => {
    fc.assert(
      fc.property(fc.string(), (name) => {
        const slot = tagSlot(name);
        expect(Number.isInteger(slot)).toBe(true);
        expect(slot).toBeGreaterThanOrEqual(1);
        expect(slot).toBeLessThanOrEqual(TAG_SLOT_COUNT);
      }),
      { numRuns: 200 },
    );
  });

  it("handles CJK and emoji names without collapsing to one slot", () => {
    // Real libraries are largely Chinese in this product; a hash that ignored
    // multi-byte code points would map every CJK tag onto the same colour.
    const names = ["工程", "测试", "重构", "思维", "职业", "写作", "学习", "决策"];
    const slots = new Set(names.map(tagSlot));
    expect(slots.size).toBeGreaterThan(1);
  });

  it("distributes a realistic tag set across most of the palette", () => {
    const names = Array.from({ length: 40 }, (_, i) => `tag-${i}`);
    const slots = new Set(names.map(tagSlot));
    expect(slots.size).toBeGreaterThanOrEqual(TAG_SLOT_COUNT - 1);
  });
});

describe("tagClasses", () => {
  it("emits literal Tailwind class names so the scanner keeps them", () => {
    // A template like `text-tag-${slot}` would be purged from the production
    // build; these must appear verbatim in the source.
    const classes = tagClasses("工程");
    expect(classes).toMatch(/text-tag-[1-8]/);
    expect(classes).toMatch(/bg-tag-[1-8]\/\d+/);
    expect(classes).not.toContain("${");
  });

  it("keeps the same hue slot between resting and pressed states", () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1 }), (name) => {
        const slot = tagSlot(name);
        expect(tagClasses(name, false)).toContain(`text-tag-${slot}`);
        expect(tagClasses(name, true)).toContain(`text-tag-${slot}`);
      }),
      { numRuns: 100 },
    );
  });

  it("gives the pressed state a stronger fill than the resting state", () => {
    expect(tagClasses("工程", false)).toContain("/15");
    expect(tagClasses("工程", true)).toContain("/25");
  });

  it("only uses alpha steps that exist on Tailwind's opacity scale", () => {
    // An off-scale modifier such as `/14` is silently dropped at build time,
    // leaving the tag with no fill at all. This regression is invisible in
    // unit tests, so assert the scale directly.
    const allowed = new Set(
      Array.from({ length: 21 }, (_, i) => String(i * 5)),
    );
    for (const name of ["工程", "测试", "重构", "思维", "职业", "写作", "学习", "决策"]) {
      for (const active of [false, true]) {
        for (const match of tagClasses(name, active).matchAll(/\/(\d+)\b/g)) {
          expect(allowed.has(match[1])).toBe(true);
        }
      }
    }
  });
});
