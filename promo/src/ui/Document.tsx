// A word processor's chrome at video scale, generic and in the product's tokens: a title bar, a tab
// row (Home active), a ribbon of text-editing tools (clipboard, font, paragraph, styles, editing) and
// a ruler, over a grey canvas. The page itself is drawn by the Language scene so its lines can leave.

import type { ReactNode } from "react";
import { C, HAIRLINE, R, WEIGHT } from "../lib/tokens";

/** Heights (px) of the chrome bands, top to bottom. */
export const CHROME = { title: 48, tabs: 46, ribbon: 112, ruler: 26 } as const;
export const CHROME_H = CHROME.title + CHROME.tabs + CHROME.ribbon + CHROME.ruler;
/** The canvas behind the page. */
export const DOC_CANVAS = "#ebecef";

function Icon({ children, size = 30, fill = false }: { children: ReactNode; size?: number; fill?: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width={size} height={size} fill={fill ? "currentColor" : "none"} stroke={fill ? "none" : "currentColor"}
      strokeWidth={1.3} strokeLinecap="round" strokeLinejoin="round">{children}</svg>
  );
}

const ICON = {
  paste: <><rect x="3" y="2.5" width="10" height="12" rx="1.2" /><rect x="5.5" y="1.5" width="5" height="2.4" rx=".6" /><path d="M5.5 7h5M5.5 9.5h5M5.5 12h3" /></>,
  cut: <><circle cx="4.5" cy="11.5" r="2" /><circle cx="11.5" cy="11.5" r="2" /><path d="M6 10 12 2.5M10 10 4 2.5" /></>,
  copy: <><rect x="5" y="5" width="8.5" height="9" rx="1" /><path d="M3 11V3a1 1 0 0 1 1-1h6" /></>,
  bullets: <><circle cx="3" cy="4" r=".9" fill="currentColor" stroke="none" /><circle cx="3" cy="8" r=".9" fill="currentColor" stroke="none" /><circle cx="3" cy="12" r=".9" fill="currentColor" stroke="none" /><path d="M6 4h8M6 8h8M6 12h8" /></>,
  numbers: <><path d="M2.4 2.8h1v3M2.2 5.8h1.6M2.2 9.2c.4-.7 1.8-.7 1.8.2 0 .8-1.8 1.3-1.8 2.2h1.9" /><path d="M6.5 4h7.5M6.5 8h7.5M6.5 12h7.5" /></>,
  outdent: <><path d="M2 3h12M7 6.5h7M7 9.5h7M2 13h12" /><path d="M4.5 6 2.5 8l2 2" /></>,
  indent: <><path d="M2 3h12M7 6.5h7M7 9.5h7M2 13h12" /><path d="M2.5 6l2 2-2 2" /></>,
  left: <path d="M2 3h12M2 6.3h8M2 9.6h12M2 13h8" />,
  center: <path d="M2 3h12M4 6.3h8M2 9.6h12M4 13h8" />,
  right: <path d="M2 3h12M6 6.3h8M2 9.6h12M6 13h8" />,
  justify: <path d="M2 3h12M2 6.3h12M2 9.6h12M2 13h12" />,
  spacing: <><path d="M7 4h7M7 8h7M7 12h7" /><path d="M3.5 2.5v11M2 4l1.5-1.5L5 4M2 12l1.5 1.5L5 12" /></>,
  find: <><circle cx="7" cy="7" r="4.2" /><path d="m10.2 10.2 3.6 3.6" /></>,
  replace: <><path d="M2.5 5.5h8l-2-2M13.5 10.5h-8l2 2" /></>,
  select: <><path d="M4 2.5v11l3-3 2 4 1.6-.8-2-4h4z" /></>,
  doc: <><path d="M4 1.5h5.5l3 3v10H4z" /><path d="M9.5 1.5v3h3M6 8h4.5M6 10.5h4.5" /></>,
  undo: <><path d="M5 4 2.5 6.5 5 9" /><path d="M2.5 6.5h7a3.5 3.5 0 0 1 0 7H7" /></>,
  save: <><path d="M3 2.5h8.5l2 2v9h-11z" /><path d="M5 2.5v3.5h5V2.5M5 13.5v-4h6v4" /></>,
};

function Btn({ children, w = 40, h = 40, on = false }: { children: ReactNode; w?: number; h?: number; on?: boolean }) {
  return (
    <div style={{ width: w, height: h, display: "flex", alignItems: "center", justifyContent: "center", borderRadius: R.xs,
      background: on ? C.soft : "transparent", boxShadow: on ? `inset 0 0 0 ${HAIRLINE}px ${C.hair}` : undefined, color: C.inkSoft }}>{children}</div>
  );
}

/** A letter-styled tool (B, I, U, S, x², x₂, the text colour and highlight A). */
function Letter({ ch, style, bar }: { ch: string; style?: React.CSSProperties; bar?: string }) {
  return (
    <Btn>
      <span style={{ position: "relative", fontSize: 24, lineHeight: 1, color: C.inkSoft, ...style }}>
        {ch}
        {bar && <span style={{ position: "absolute", left: -3, right: -3, bottom: -7, height: 5, background: bar, borderRadius: 1 }} />}
      </span>
    </Btn>
  );
}

function Drop({ text, w }: { text: string; w: number }) {
  return (
    <div style={{ width: w, height: 38, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 12px", boxSizing: "border-box",
      background: C.canvas, border: `${HAIRLINE}px solid ${C.hair}`, borderRadius: R.sm, fontSize: 22, color: C.ink }}>
      {text}
      <svg width={14} height={14} viewBox="0 0 16 16"><path d="M3.5 6 8 10.5 12.5 6" fill="none" stroke={C.muted} strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" /></svg>
    </div>
  );
}

function Group({ label, children, w }: { label: string; children: ReactNode; w?: number }) {
  return (
    <div style={{ width: w, height: "100%", display: "flex", flexDirection: "column", justifyContent: "space-between", padding: "10px 20px 8px",
      boxSizing: "border-box", borderRight: `${HAIRLINE}px solid ${C.hair}` }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>{children}</div>
      <div style={{ fontSize: 17, color: C.muted, textAlign: "center" }}>{label}</div>
    </div>
  );
}

const Row = ({ children }: { children: ReactNode }) => <div style={{ display: "flex", alignItems: "center", gap: 2 }}>{children}</div>;

/** A style in the gallery: its sample, then its name. */
function Style({ name, sample, on = false }: { name: string; sample: React.CSSProperties & { text: string }; on?: boolean }) {
  const { text, ...st } = sample;
  return (
    <div style={{ width: 120, height: 70, borderRadius: R.xs, border: `${HAIRLINE}px solid ${on ? C.ink : C.hair}`, background: C.canvas,
      display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 4 }}>
      <span style={{ fontSize: 22, color: C.ink, lineHeight: 1, ...st }}>{text}</span>
      <span style={{ fontSize: 15, color: C.muted }}>{name}</span>
    </div>
  );
}

/** The chrome bands for a document called `title`, `width` px wide (the canvas below is the caller's). */
export function DocChrome({ title, width, pageW, margin, font = "Inter", size = "12" }: { title: string; width: number; pageW: number; margin: number; font?: string; size?: string }) {
  const tabs = ["File", "Home", "Insert", "Layout", "References", "Review", "View"];
  return (
    <div style={{ width, background: C.canvas, color: C.ink }}>
      {/* title bar: quick actions, the document name, its state */}
      <div style={{ height: CHROME.title, display: "flex", alignItems: "center", padding: "0 20px", gap: 18, background: C.soft, color: C.inkSoft }}>
        <Icon size={26}>{ICON.doc}</Icon><Icon size={24}>{ICON.save}</Icon><Icon size={24}>{ICON.undo}</Icon>
        <div style={{ flex: 1, textAlign: "center", fontSize: 21 }}>
          <span style={{ fontWeight: WEIGHT.head }}>{title}</span><span style={{ color: C.muted }}> · Saved</span>
        </div>
        <div style={{ width: 96 }} />
      </div>
      {/* tabs */}
      <div style={{ height: CHROME.tabs, display: "flex", alignItems: "flex-end", gap: 38, padding: "0 28px", borderBottom: `${HAIRLINE}px solid ${C.hair}` }}>
        {tabs.map((t) => (
          <div key={t} style={{ paddingBottom: 9, fontSize: 21, fontWeight: t === "Home" ? WEIGHT.head : WEIGHT.text, color: t === "Home" ? C.ink : C.muted,
            borderBottom: `3px solid ${t === "Home" ? C.ink : "transparent"}` }}>{t}</div>
        ))}
      </div>
      {/* ribbon */}
      <div style={{ height: CHROME.ribbon, display: "flex", borderBottom: `${HAIRLINE}px solid ${C.hair}` }}>
        <Group label="Clipboard">
          <Row>
            <Btn w={54} h={58}><Icon size={40}>{ICON.paste}</Icon></Btn>
            <div style={{ display: "flex", flexDirection: "column" }}><Btn h={30}><Icon size={22}>{ICON.cut}</Icon></Btn><Btn h={30}><Icon size={22}>{ICON.copy}</Icon></Btn></div>
          </Row>
        </Group>
        <Group label="Font">
          <Row><Drop text={font} w={200} /><div style={{ width: 8 }} /><Drop text={size} w={78} /></Row>
          <Row>
            <Letter ch="B" style={{ fontWeight: 750 }} /><Letter ch="I" style={{ fontStyle: "italic", fontFamily: "serif" }} /><Letter ch="U" style={{ textDecoration: "underline" }} />
            <Letter ch="S" style={{ textDecoration: "line-through" }} /><Letter ch="x₂" style={{ fontSize: 20 }} /><Letter ch="x²" style={{ fontSize: 20 }} />
            <Letter ch="A" bar={C.error} /><Letter ch="A" bar={C.warning} style={{ fontSize: 20 }} />
          </Row>
        </Group>
        <Group label="Paragraph">
          <Row>{[ICON.bullets, ICON.numbers, ICON.outdent, ICON.indent].map((d, i) => <Btn key={i}><Icon size={26}>{d}</Icon></Btn>)}</Row>
          <Row>{[ICON.left, ICON.center, ICON.right, ICON.justify, ICON.spacing].map((d, i) => <Btn key={i} on={i === 0}><Icon size={26}>{d}</Icon></Btn>)}</Row>
        </Group>
        <Group label="Styles">
          <Row>
            <Style name="Normal" sample={{ text: "AaBb" }} on />
            <Style name="No spacing" sample={{ text: "AaBb" }} />
            <Style name="Heading 1" sample={{ text: "AaBb", fontWeight: WEIGHT.head, fontSize: 24 }} />
            <Style name="Title" sample={{ text: "Aa", fontWeight: WEIGHT.head, fontSize: 30 }} />
          </Row>
        </Group>
        <Group label="Editing">
          {[["find", "Find"], ["replace", "Replace"], ["select", "Select"]].map(([k, t]) => (
            <div key={k} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 18, color: C.inkSoft, height: 25 }}>
              <Icon size={20}>{ICON[k as keyof typeof ICON]}</Icon>{t}
            </div>
          ))}
        </Group>
        <div style={{ flex: 1 }} />
      </div>
      {/* ruler */}
      <div style={{ height: CHROME.ruler, background: DOC_CANVAS, position: "relative", borderBottom: `${HAIRLINE}px solid ${C.hair}` }}>
        <Ruler width={width} pageW={pageW} margin={margin} />
      </div>
    </div>
  );
}

/** The ruler's marks over a page `pageW` wide centred in `width`, text area inset by `margin`. */
function Ruler({ width, pageW, margin }: { width: number; pageW: number; margin: number }) {
  const x0 = (width - pageW) / 2, step = pageW / 34;
  return (
    <svg width={width} height={CHROME.ruler} style={{ position: "absolute", left: 0, top: 0 }}>
      <rect x={x0} y={3} width={pageW} height={CHROME.ruler - 6} fill={C.hairSoft} />
      <rect x={x0 + margin} y={3} width={pageW - 2 * margin} height={CHROME.ruler - 6} fill={C.canvas} />
      {Array.from({ length: 35 }, (_, i) => (
        <line key={i} x1={x0 + i * step} x2={x0 + i * step} y1={i % 4 === 0 ? 8 : 13} y2={CHROME.ruler - 8} stroke={C.faint} strokeWidth={1.2} />
      ))}
      <path d={`M${x0 + margin - 7},4 h14 l-7,7 z`} fill={C.muted} />
      <path d={`M${x0 + pageW - margin - 7},${CHROME.ruler - 4} h14 l-7,-7 z`} fill={C.muted} />
    </svg>
  );
}
