/**
 * Entry-point tests for the Appearance_Controller production wiring (Task 6.2).
 * With an injected bridge spy: each set* applies to the controller BEFORE
 * issuing `settings.update`, and a rejecting bridge keeps the applied value in
 * current()/DOM without letting an exception escape (Req 5.6).
 *
 * Driven through a fake injected root, so these stay in the Node environment.
 */
import { describe, expect, it, vi } from "vitest";
import fc from "fast-check";
import { BridgeError } from "../runtime";
import {
  ACCENT_PALETTE,
  createAppearanceController,
  FLAVOR_OVERRIDES,
  FONT_STACK,
  setAccentColor,
  setBodyFont,
  setDensity,
  setDisplayFont,
  setFlavor,
  setFontScale,
  type Appearance,
  type AppearanceController,
  type AppearanceDeps,
} from "./index";

/** The injectable `invoke` type the set* entry points accept. */
type Invoke = NonNullable<Parameters<typeof setFlavor>[1]>;

/** A fake root recording class toggles, CSS variables, and attributes. */
function makeFakeRoot() {
  const classes = new Set<string>();
  const vars = new Map<string, string>();
  const attrs = new Map<string, string>();
  const root: AppearanceDeps["root"] = {
    classList: { add: (t) => void classes.add(t), remove: (t) => void classes.delete(t) },
    style: { setProperty: (name, value) => void vars.set(name, value) },
    setAttribute: (name, value) => void attrs.set(name, value),
  };
  return { root, classes, vars, attrs };
}

/** A controller wrapper that records the order of apply vs persist. */
function makeProbe() {
  const fake = makeFakeRoot();
  const real = createAppearanceController({ root: fake.root });
  const order: string[] = [];
  const controller: AppearanceController = {
    apply: (a) => {
      order.push("apply");
      real.apply(a);
    },
    applyField: (f, v) => {
      order.push("apply");
      real.applyField(f, v);
    },
    current: () => real.current(),
  };
  return { fake, controller, order };
}

interface Case {
  name: string;
  field: keyof Appearance;
  value: Appearance[keyof Appearance];
  run: (invoke: Invoke, controller: AppearanceController) => Promise<void>;
  dom: (fake: ReturnType<typeof makeFakeRoot>) => void;
}

const CASES: Case[] = [
  {
    name: "setFlavor",
    field: "flavor",
    value: "Latte",
    run: (i, c) => setFlavor("Latte", i, c),
    dom: (f) => expect(f.vars.get("--background")).toBe(FLAVOR_OVERRIDES.Latte["--background"]),
  },
  {
    name: "setAccentColor",
    field: "accentColor",
    value: "Green",
    run: (i, c) => setAccentColor("Green", i, c),
    // The default controller flavor (Mocha) is a dark base.
    dom: (f) => expect(f.vars.get("--primary")).toBe(ACCENT_PALETTE.dark.Green["--primary"]),
  },
  {
    name: "setDisplayFont",
    field: "displayFont",
    value: "Inter",
    run: (i, c) => setDisplayFont("Inter", i, c),
    dom: (f) => expect(f.vars.get("--font-display")).toBe(FONT_STACK.Inter),
  },
  {
    name: "setBodyFont",
    field: "bodyFont",
    value: "Space Grotesk",
    run: (i, c) => setBodyFont("Space Grotesk", i, c),
    dom: (f) => expect(f.vars.get("--font-body")).toBe(FONT_STACK["Space Grotesk"]),
  },
  {
    name: "setFontScale",
    field: "fontScale",
    value: "Large",
    run: (i, c) => setFontScale("Large", i, c),
    dom: (f) => expect(f.vars.get("--font-scale")).toBe("1.1"),
  },
  {
    name: "setDensity",
    field: "density",
    value: "Comfortable",
    run: (i, c) => setDensity("Comfortable", i, c),
    dom: (f) => expect(f.attrs.get("data-density")).toBe("Comfortable"),
  },
];

describe("Appearance entry points (Task 6.2)", () => {
  it("applies to the controller before issuing settings.update, with the right patch", async () => {
    await fc.assert(
      fc.asyncProperty(fc.constantFrom(...CASES), async (c) => {
        const { controller, order } = makeProbe();
        const invoke = vi.fn(async () => {
          order.push("invoke");
          return undefined;
        });
        await c.run(invoke as unknown as Invoke, controller);
        // Instant-apply happens first, persistence second.
        expect(order).toEqual(["apply", "invoke"]);
        expect(invoke).toHaveBeenCalledTimes(1);
        expect(invoke).toHaveBeenCalledWith("settings.update", { patch: { [c.field]: c.value } });
      }),
      { numRuns: 100 },
    );
  });

  it("keeps the applied value in current()/DOM and lets no exception escape on a rejecting bridge", async () => {
    await fc.assert(
      fc.asyncProperty(fc.constantFrom(...CASES), async (c) => {
        const { fake, controller } = makeProbe();
        const invoke = vi.fn(async () => {
          throw new BridgeError("INTERNAL", "persist failed");
        });
        // No exception escapes the set* call.
        await expect(c.run(invoke as unknown as Invoke, controller)).resolves.toBeUndefined();
        // The applied value remains for the session, in state and in the DOM.
        expect(controller.current()[c.field]).toBe(c.value);
        c.dom(fake);
      }),
      { numRuns: 100 },
    );
  });
});
