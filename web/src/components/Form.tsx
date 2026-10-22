// Form building blocks for the input sidebars (context/design.md "Section", rows) and the
// three-panel frame every input tab and Home use (web-screens.md "Shell").

import { useState, type ReactNode } from "react";

/** Sidebar (title row, scrolling body, optional footer) | main visual over a bottom panel. */
export function Frame({ title, control, side, footer, main, bottom, sideRef, onSideScroll }: {
  title: ReactNode; control?: ReactNode; side: ReactNode; footer?: ReactNode; main: ReactNode; bottom: ReactNode;
  sideRef?: React.Ref<HTMLDivElement>; onSideScroll?: () => void;
}) {
  return (
    <div className="frame">
      <aside className="frame-side">
        <div className="panehead"><b>{title}</b>{control && <span className="headctl">{control}</span>}</div>
        <div className="scroll" ref={sideRef} onScroll={onSideScroll}>{side}</div>
        {footer && <div className="frame-foot">{footer}</div>}
      </aside>
      <section className="frame-main">{main}</section>
      <section className="frame-bottom">{bottom}</section>
    </div>
  );
}

/** A collapsible section; open state remembered per browser. */
export function Sec({ id, title, note, children, defaultOpen = true }: { id: string; title: string; note?: string; children: ReactNode; defaultOpen?: boolean }) {
  const key = `gecko.sec.${id}`;
  const [open, setOpen] = useState(() => {
    try { const v = localStorage.getItem(key); return v === null ? defaultOpen : v !== "0"; } catch { return defaultOpen; }
  });
  return (
    <details className="sec" open={open} onToggle={(e) => {
      const o = (e.target as HTMLDetailsElement).open;
      setOpen(o);
      try { localStorage.setItem(key, o ? "1" : "0"); } catch { /* storage unavailable */ }
    }}>
      <summary>{title}{note && <span className="note">{note}</span>}</summary>
      <div className="body">{children}</div>
    </details>
  );
}

export function Num({ label, help, value, onChange, unit, step = 1, min, max, disabled }: {
  label: string; help: string; value: number; onChange: (v: number) => void; unit?: string; step?: number; min?: number; max?: number; disabled?: boolean;
}) {
  return (
    <div className="row">
      <label title={help}>{label}</label>
      <input type="number" value={Number.isFinite(value) ? +value.toPrecision(6) : ""} step={step} min={min} max={max} disabled={disabled} title={help}
        onChange={(e) => { const v = parseFloat(e.target.value); if (Number.isFinite(v)) onChange(v); }} />
      <span className="unit">{unit}</span>
    </div>
  );
}

export function Slider({ label, help, value, onChange, min, max, step, unit, disabled }: {
  label: string; help: string; value: number; onChange: (v: number) => void; min: number; max: number; step: number; unit?: string; disabled?: boolean;
}) {
  return (
    <div className="row">
      <label title={help}>{label}</label>
      <input type="range" value={value} min={min} max={max} step={step} disabled={disabled} title={help} onChange={(e) => onChange(parseFloat(e.target.value))} />
      <span className="unit">{unit ?? value}</span>
    </div>
  );
}

export function Toggle({ label, help, value, onChange, disabled }: { label: string; help: string; value: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <div className="row">
      <label title={help}>{label}</label>
      <input type="checkbox" className="switch" checked={value} disabled={disabled} title={help} onChange={(e) => onChange(e.target.checked)} />
      <span />
    </div>
  );
}

/** Segmented control. */
export function Seg<T extends string>({ value, options, onChange, disabled, inline }: {
  value: T; options: [T, string][]; onChange: (v: T) => void; disabled?: boolean; inline?: boolean;
}) {
  return (
    <div className={`seg ${inline ? "inline" : ""}`}>
      {options.map(([v, n]) => (
        <button key={v} className={value === v ? "on" : ""} aria-pressed={value === v} disabled={disabled} onClick={() => onChange(v)}>{n}</button>
      ))}
    </div>
  );
}

/** Tab selector for a bottom panel (top-left). */
export function PanelTabs<T extends string>({ value, options, onChange, right }: { value: T; options: [T, string][]; onChange: (v: T) => void; right?: ReactNode }) {
  return (
    <div className="paneltabs">
      <Seg inline value={value} options={options} onChange={onChange} />
      {right && <span className="right">{right}</span>}
    </div>
  );
}
