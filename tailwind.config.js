/** @type {import('tailwindcss').Config} */
export default {
  // Theme switching is driven by a `.dark` class on <html> (see src/theme/index.ts).
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      // Utilities resolve to the CSS custom properties (design tokens) declared
      // in src/styles/globals.css. The channel-only HSL values let callers add an
      // alpha, e.g. `bg-primary/20` -> `hsl(var(--primary) / 0.2)`.
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
          "foreground-subtle": "hsl(var(--muted-foreground-subtle))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        sidebar: {
          DEFAULT: "hsl(var(--sidebar))",
          foreground: "hsl(var(--sidebar-foreground))",
          accent: "hsl(var(--sidebar-accent))",
          border: "hsl(var(--sidebar-border))",
        },
        success: "hsl(var(--success))",
        warning: "hsl(var(--warning))",
        "accent-alt": "hsl(var(--accent-alt))",
        "diff-added": "hsl(var(--diff-added))",
        "diff-removed": "hsl(var(--diff-removed))",
        "border-strong": "hsl(var(--border-strong))",
        "surface-inset": "hsl(var(--surface-inset))",
        // Interaction states: one token per state instead of ad-hoc
        // `bg-primary/10` / `bg-primary/15` / `hover:bg-accent`.
        "state-hover": "hsl(var(--state-hover))",
        "state-selected": "hsl(var(--state-selected))",
        "state-pressed": "hsl(var(--state-pressed))",
        favorite: "hsl(var(--favorite))",
        // Global tag palette; `Tag` maps a tag name to one of these slots.
        "tag-1": "hsl(var(--tag-1))",
        "tag-2": "hsl(var(--tag-2))",
        "tag-3": "hsl(var(--tag-3))",
        "tag-4": "hsl(var(--tag-4))",
        "tag-5": "hsl(var(--tag-5))",
        "tag-6": "hsl(var(--tag-6))",
        "tag-7": "hsl(var(--tag-7))",
        "tag-8": "hsl(var(--tag-8))",
      },
      fontFamily: {
        mono: ["var(--font-mono)"],
      },
      /*
       * Six-step type scale. Each step carries its own line-height and tracking
       * so hierarchy comes from the token, not from pairing `text-xs` with a
       * hand-picked `leading-*`. Replaces the former text-xs/text-sm duopoly and
       * the `text-[10px]` / `text-[11px]` arbitrary values.
       */
      fontSize: {
        micro: ["10px", { lineHeight: "14px", letterSpacing: "0.04em" }],
        meta: ["11px", { lineHeight: "16px", letterSpacing: "0.02em" }],
        label: ["12px", { lineHeight: "16px", letterSpacing: "0.01em" }],
        body: ["13px", { lineHeight: "20px", letterSpacing: "0" }],
        title: ["15px", { lineHeight: "22px", letterSpacing: "-0.006em" }],
        display: ["19px", { lineHeight: "26px", letterSpacing: "-0.012em" }],
        hero: ["24px", { lineHeight: "32px", letterSpacing: "-0.018em" }],
      },
      // Control heights: one row of chrome must use a single step.
      height: {
        "control-xs": "var(--control-xs)",
        "control-sm": "var(--control-sm)",
        "control-md": "var(--control-md)",
        "control-lg": "var(--control-lg)",
      },
      width: {
        "control-xs": "var(--control-xs)",
        "control-sm": "var(--control-sm)",
        "control-md": "var(--control-md)",
        "control-lg": "var(--control-lg)",
      },
      transitionTimingFunction: {
        out: "var(--ease-out)",
        spring: "var(--ease-spring)",
      },
      transitionDuration: {
        fast: "var(--dur-fast)",
        base: "var(--dur-base)",
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      boxShadow: {
        sm: "var(--shadow-sm)",
        DEFAULT: "var(--shadow)",
        lg: "var(--shadow-lg)",
        // Edge highlight (dark) / faint bottom inset (light). Not a drop shadow.
        hairline: "var(--hairline)",
        overlay: "var(--shadow-overlay)",
        panel: "var(--shadow-panel)",
      },
    },
  },
  plugins: [],
};
