import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";

describe("Kinetic Typography and Entrance Animations", () => {
  let originalWindow: unknown;
  let originalDocument: unknown;

  before(() => {
    originalWindow = (globalThis as unknown as { window?: unknown }).window;
    originalDocument = (globalThis as unknown as { document?: unknown }).document;
  });

  after(() => {
    (globalThis as unknown as { window?: unknown }).window = originalWindow;
    (globalThis as unknown as { document?: unknown }).document = originalDocument;
  });

  it("splitType splits headings into words and letters with .kin, .wd, .lt", async () => {
    // Setup a lightweight mock DOM environment
    class MockElement {
      tagName: string;
      className: string;
      classList = {
        contains: (c: string) => this.className.split(" ").includes(c),
        add: (c: string) => {
          if (!this.classList.contains(c)) this.className = (this.className + " " + c).trim();
        },
      };
      attributes: Record<string, string> = {};
      styleProps: Record<string, string> = {};
      style = {
        width: "",
        transform: "",
        fontWeight: "",
        getPropertyValue: (name: string) => this.styleProps[name] || "",
        setProperty: (name: string, val: string) => {
          this.styleProps[name] = val;
        },
      };
      children: MockElement[] = [];
      parentElement: MockElement | null = null;
      _textContent = "";
      _innerHTML = "";
      isConnected = true;

      constructor(tagName: string, className = "") {
        this.tagName = tagName;
        this.className = className;
      }

      get textContent(): string {
        return this._textContent;
      }
      set textContent(val: string) {
        this._textContent = val;
      }

      get innerHTML(): string {
        return this._innerHTML;
      }
      set innerHTML(html: string) {
        this._innerHTML = html;
        this.children = [];
        const letterMatches = [...html.matchAll(/<span class="lt">(.*?)<\/span>/g)];
        for (const lm of letterMatches) {
          const lt = new MockElement("SPAN", "lt");
          lt.textContent = lm[1];
          lt.parentElement = this;
          this.children.push(lt);
        }
      }

      setAttribute(name: string, value: string) {
        this.attributes[name] = value;
      }
      getAttribute(name: string) {
        return this.attributes[name];
      }
      closest(sel: string): MockElement | null {
        if (sel === "[aria-hidden]" && this.attributes["aria-hidden"] === "true") return this;
        return this.parentElement?.closest(sel) || null;
      }
      querySelector(sel: string): MockElement | null {
        const all = this.querySelectorAll(sel);
        return all.length > 0 ? all[0] : null;
      }
      querySelectorAll(sel: string): MockElement[] {
        const res: MockElement[] = [];
        const check = (el: MockElement) => {
          if (sel === ".lt" && el.classList.contains("lt")) res.push(el);
          if (sel === ".wd" && el.classList.contains("wd")) res.push(el);
          if (sel === ".in" && el.classList.contains("in")) res.push(el);
          if (
            (sel === ".h1, .bigh" || sel === ".h1" || sel === ".bigh") &&
            (el.classList.contains("h1") || el.classList.contains("bigh"))
          ) {
            res.push(el);
          }
          el.children.forEach(check);
        };
        this.children.forEach(check);
        return res;
      }
      getBoundingClientRect() {
        return { left: 100, top: 50, width: 24, height: 32 };
      }
    }

    const mockWindow = {
      matchMedia: () => ({ matches: false }),
      addEventListener: () => {},
      removeEventListener: () => {},
      requestAnimationFrame: () => 1,
      cancelAnimationFrame: () => {},
      getComputedStyle: () => ({
        fontSize: "32px",
        getPropertyValue: (prop: string) => {
          if (prop === "--kin-lo") return "640";
          if (prop === "--kin-hi") return "900";
          return "";
        },
      }),
    };

    (globalThis as unknown as { window: unknown }).window = mockWindow;
    (globalThis as unknown as { document: unknown }).document = {
      documentElement: new MockElement("HTML"),
      fonts: { ready: Promise.resolve() },
    };

    const { splitType, staggerEntrance, setKineticPlaying } = await import("../kinetic.js");

    const root = new MockElement("DIV");
    const heading = new MockElement("H1", "h1");
    heading.textContent = "Flux Editor";
    heading.parentElement = root;
    root.children.push(heading);

    const letters = splitType(root as unknown as HTMLElement);

    assert.equal(heading.getAttribute("aria-label"), "Flux Editor");
    assert.ok(heading.classList.contains("kin"));
    assert.equal(letters.length, 10); // "Flux" (4) + "Editor" (6)
    assert.equal(letters[0].i, 0);
    assert.equal(letters[9].i, 9);
    assert.ok(letters[0].l.style.width.endsWith("em"));

    // Stagger entrance test
    const inContainer = new MockElement("DIV", "pg in");
    inContainer.parentElement = root;
    const c1 = new MockElement("DIV");
    const c2 = new MockElement("DIV");
    const c3 = new MockElement("DIV");
    inContainer.children.push(c1, c2, c3);
    root.children.push(inContainer);

    staggerEntrance(root as unknown as HTMLElement);

    assert.equal(c1.style.getPropertyValue("--n"), "0");
    assert.equal(c2.style.getPropertyValue("--n"), "1");
    assert.equal(c3.style.getPropertyValue("--n"), "2");

    setKineticPlaying(true);
  });
});
