// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Button } from "./Button";
import { IconButton } from "./IconButton";
import { Input } from "./Input";
import { Select } from "./Select";
import { Tag } from "./Tag";
import { UsageBar } from "./UsageBar";
import { EmptyState } from "./EmptyState";

afterEach(cleanup);

describe("Button", () => {
  it("defaults to type=button so it never submits a surrounding form", () => {
    const onSubmit = vi.fn((event: React.FormEvent) => event.preventDefault());
    render(
      <form onSubmit={onSubmit}>
        <Button>Save</Button>
      </form>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("exposes a single disabled opacity and blocks pointer events", () => {
    render(<Button disabled>Save</Button>);
    const button = screen.getByRole("button", { name: "Save" });
    expect(button.hasAttribute("disabled")).toBe(true);
    expect(button.className).toContain("disabled:opacity-50");
    expect(button.className).toContain("disabled:pointer-events-none");
  });

  it("relies on the base-layer focus ring rather than a per-component one", () => {
    // The ring now lives in a single `:focus-visible` rule in globals.css so
    // that every control gets one; duplicating it here would paint an outline
    // and a ring at the same time.
    render(<Button>Save</Button>);
    expect(screen.getByRole("button").className).not.toContain("focus-visible:ring");
  });

  it("routes radius through the token scale rather than a bare rounded", () => {
    render(<Button>Save</Button>);
    const classes = screen.getByRole("button").className.split(/\s+/);
    expect(classes).toContain("rounded-md");
    expect(classes).not.toContain("rounded");
  });
});

describe("IconButton", () => {
  it("labels the control for assistive technology and as a tooltip", () => {
    render(<IconButton label="Copy prompt" icon={<svg />} />);
    const button = screen.getByRole("button", { name: "Copy prompt" });
    expect(button.getAttribute("title")).toBe("Copy prompt");
  });

  it("uses a token control size instead of an ad-hoc height", () => {
    render(<IconButton label="Copy" icon={<svg />} size="sm" />);
    expect(screen.getByRole("button").className).toContain("h-control-sm");
  });
});

describe("Input", () => {
  it("keeps the accessible name and forwards typing", () => {
    const onChange = vi.fn();
    render(<Input aria-label="Search" onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "abc" } });
    expect(onChange).toHaveBeenCalled();
  });

  it("marks invalid fields with aria-invalid", () => {
    render(<Input aria-label="Title" invalid />);
    expect(screen.getByLabelText("Title").getAttribute("aria-invalid")).toBe("true");
  });
});

describe("Select", () => {
  it("renders a real select so keyboard and screen-reader behaviour is native", () => {
    render(
      <Select
        aria-label="Sort"
        value="a"
        onChange={() => {}}
        options={[
          { value: "a", label: "Alpha" },
          { value: "b", label: "Beta" },
        ]}
      />,
    );
    const select = screen.getByRole("combobox", { name: "Sort" });
    expect(select.tagName).toBe("SELECT");
    expect(screen.getByRole("option", { name: "Beta" })).toBeDefined();
  });

  it("reports the chosen value to the caller", () => {
    const onChange = vi.fn();
    render(
      <Select
        aria-label="Sort"
        value="a"
        onChange={onChange}
        options={[
          { value: "a", label: "Alpha" },
          { value: "b", label: "Beta" },
        ]}
      />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "b" } });
    expect(onChange).toHaveBeenCalled();
  });
});

describe("Tag", () => {
  it("renders static tags as non-interactive text", () => {
    render(<Tag name="工程" />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("工程")).toBeDefined();
  });

  it("exposes selection through aria-pressed, not colour alone", () => {
    const onToggle = vi.fn();
    render(<Tag name="测试" pressed onToggle={onToggle} />);
    const button = screen.getByRole("button", { name: /测试/ });
    expect(button.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(button);
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("gives the same tag the same hue in every render site", () => {
    const { container: a } = render(<Tag name="重构" />);
    const { container: b } = render(<Tag name="重构" count={3} onToggle={() => {}} />);
    const hue = /text-tag-([1-8])/;
    const first = a.firstElementChild?.className.match(hue)?.[1];
    const second = b.firstElementChild?.className.match(hue)?.[1];
    expect(first).toBeDefined();
    expect(second).toBe(first);
  });
});

describe("UsageBar", () => {
  it("describes the magnitude for assistive technology", () => {
    render(<UsageBar value={12} max={24} label="Usage: 12" />);
    expect(screen.getByRole("img", { name: "Usage: 12" })).toBeDefined();
  });

  it("clamps out-of-range values instead of overflowing the track", () => {
    const { container } = render(<UsageBar value={99} max={10} label="Usage" />);
    const fill = container.querySelector<HTMLElement>("[style*='width']");
    expect(fill?.style.width).toBe("100%");
  });

  it("survives a zero maximum without dividing by zero", () => {
    const { container } = render(<UsageBar value={0} max={0} label="Usage" />);
    const fill = container.querySelector<HTMLElement>("[style*='width']");
    expect(fill?.style.width).toBe("0%");
  });

  it("renders the exact count in tabular figures", () => {
    render(<UsageBar value={7} max={10} label="Usage" />);
    expect(screen.getByText("7").className).toContain("tabular-nums");
  });
});

describe("EmptyState", () => {
  it("shows a title, an explanation, and a recovery action", () => {
    render(
      <EmptyState
        title="No prompts"
        description="Try clearing the active filters."
        action={<Button>Clear all</Button>}
      />,
    );
    expect(screen.getByText("No prompts")).toBeDefined();
    expect(screen.getByText("Try clearing the active filters.")).toBeDefined();
    expect(screen.getByRole("button", { name: "Clear all" })).toBeDefined();
  });

  it("forwards data attributes so roving-focus navigation can find the chip", () => {
    // PromptLibraryNav drives ArrowLeft/ArrowRight over the tag cloud by
    // querying [data-tag-chip]; dropping the attribute would silently kill
    // keyboard navigation without failing any render assertion.
    const { container } = render(
      <Tag name="工程" data-tag-chip="" onToggle={vi.fn()} />,
    );
    expect(container.querySelectorAll("[data-tag-chip]").length).toBe(1);
  });

  it("labels the count for assistive technology", () => {
    render(<Tag name="工程" count={12} countLabel="12 prompts" />);
    expect(screen.getByLabelText("12 prompts").textContent).toBe("12");
  });
});
