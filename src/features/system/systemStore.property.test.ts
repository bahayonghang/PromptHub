import { describe, expect, it } from "vitest";
import fc from "fast-check";
import { downloadProgressPercent } from "./systemStore";

/**
 * Property coverage for the updater download-progress percentage (Req 24.3).
 *
 * `downloadProgressPercent` converts the cumulative `downloaded` / `total` byte
 * counts carried by `updater:status` events into a bounded UI percentage. The
 * properties below assert the invariants the progress bar relies on across the
 * whole input space rather than a handful of examples.
 *
 * **Validates: Requirements 24.3**
 */
describe("downloadProgressPercent properties (Req 24.3)", () => {
  it("always yields an integer within [0, 100] when total is positive", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 1_000_000_000 }),
        fc.integer({ min: 1, max: 1_000_000_000 }),
        (downloaded, total) => {
          const percent = downloadProgressPercent(downloaded, total);
          expect(percent).not.toBeNull();
          const p = percent as number;
          expect(Number.isInteger(p)).toBe(true);
          expect(p).toBeGreaterThanOrEqual(0);
          expect(p).toBeLessThanOrEqual(100);
        },
      ),
    );
  });

  it("returns null whenever the total is unknown or non-positive", () => {
    fc.assert(
      fc.property(
        fc.option(fc.integer({ min: 0, max: 1_000_000 }), { nil: null }),
        fc.integer({ min: -1_000_000, max: 0 }),
        (downloaded, nonPositiveTotal) => {
          expect(downloadProgressPercent(downloaded, null)).toBeNull();
          expect(downloadProgressPercent(downloaded, nonPositiveTotal)).toBeNull();
        },
      ),
    );
  });

  it("is monotonic non-decreasing as more bytes arrive for a fixed total", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1_000_000 }),
        fc.integer({ min: 0, max: 1_000_000 }),
        fc.integer({ min: 0, max: 1_000_000 }),
        (total, a, b) => {
          const lo = Math.min(a, b);
          const hi = Math.max(a, b);
          const pLo = downloadProgressPercent(lo, total) as number;
          const pHi = downloadProgressPercent(hi, total) as number;
          expect(pHi).toBeGreaterThanOrEqual(pLo);
        },
      ),
    );
  });
});
