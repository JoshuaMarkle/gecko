// Profile tab, "Fit to layout" section: the weights under which the current layout is closest to
// optimal (POST /api/fit-layout, gecko-core `inverse`), a chart of the weight multipliers, and the
// reference layouts re-ranked under the fitted weights.

import { type FitResponse, type MetricDef } from "../api";
import { fmt } from "../format";
import { Num, Seg } from "./Form";

export type FitBudget = "quick" | "standard" | "thorough";
export const FIT_BUDGETS: Record<FitBudget, { rounds: number; evals: number; seeds: number }> = {
  quick: { rounds: 3, evals: 1_000_000, seeds: 1 },
  standard: { rounds: 6, evals: 4_000_000, seeds: 2 },
  thorough: { rounds: 10, evals: 10_000_000, seeds: 3 },
};

export function FitControls({ budget, setBudget, sigma, setSigma, busy, onFit, res, onApply }: {
  budget: FitBudget; setBudget: (b: FitBudget) => void; sigma: number; setSigma: (v: number) => void;
  busy: boolean; onFit: () => void; res: FitResponse | null; onApply: () => void;
}) {
  return (
    <>
      <div className="row">
        <label title="Search effort per fit: rounds of search under the fitted weights, evaluations and seeds per round.">Budget</label>
        <Seg value={budget} options={[["quick", "Quick"], ["standard", "Standard"], ["thorough", "Thorough"]]} onChange={setBudget} disabled={busy} />
        <span />
      </div>
      <Num label="Prior σ" help="How far each weight may move from this profile: log-normal σ of each multiplier (0.5 ≈ ×0.37–×2.7 at 95 %). Larger lets the layout pull weights further."
        value={sigma} step={0.1} min={0.1} max={3} onChange={setSigma} disabled={busy} />
      <div className="btns">
        <button className="btn primary small" disabled={busy} onClick={onFit}>{busy ? "Fitting…" : "Fit weights to this layout"}</button>
        <button className="btn soft small" disabled={busy || !res} onClick={onApply} title="Replace this profile's weights with the fitted ones (edits the profile; Reset restores it).">Apply fitted weights</button>
      </div>
    </>
  );
}

/** Log multipliers with 95 % intervals, largest change first. */
export function FitChart({ res, metrics }: { res: FitResponse | null; metrics: MetricDef[] }) {
  if (!res) return <div className="empty">Fit the weights to see which preferences make this layout optimal.</div>;
  const name = (id: string) => metrics.find((m) => m.id === id)?.name ?? id;
  const ws = [...res.fit.weights].sort((a, b) => Math.abs(Math.log(b.multiplier)) - Math.abs(Math.log(a.multiplier)));
  const lim = Math.log(8);
  const W = 640, row = 18, left = 170, right = 70, top = 22;
  const H = top + ws.length * row + 8;
  const x = (m: number) => left + ((Math.max(-lim, Math.min(lim, Math.log(m))) + lim) / (2 * lim)) * (W - left - right);
  const ticks = [0.125, 0.25, 0.5, 1, 2, 4, 8];
  return (
    <div className="visual">
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ maxWidth: W }} role="img" aria-label="Fitted weight multipliers">
        {ticks.map((t) => (
          <g key={t}>
            <line x1={x(t)} x2={x(t)} y1={top - 4} y2={H - 6} stroke={t === 1 ? "var(--ink)" : "var(--hair)"} />
            <text x={x(t)} y={top - 8} textAnchor="middle" fontSize={10} fill="var(--muted)">×{t}</text>
          </g>
        ))}
        {ws.map((w, i) => {
          const y = top + i * row + row / 2;
          const prior = w.sd_ratio > 0.8;
          const ink = prior ? "var(--faint)" : "var(--ink)";
          return (
            <g key={w.id}>
              <text x={left - 8} y={y + 4} textAnchor="end" fontSize={11} fill={prior ? "var(--muted)" : "var(--ink)"}>{name(w.id)}</text>
              <line x1={x(w.lo)} x2={x(w.hi)} y1={y} y2={y} stroke={ink} strokeWidth={1} />
              <rect x={Math.min(x(1), x(w.multiplier))} y={y - 4} width={Math.abs(x(w.multiplier) - x(1))} height={8} fill={ink} opacity={prior ? 0.5 : 0.85} />
              <text x={W - right + 6} y={y + 4} fontSize={11} fill="var(--muted)" style={{ fontVariantNumeric: "tabular-nums" }}>
                ×{fmt(w.multiplier, 2)}{w.at_bound ? " ⟂" : ""}
              </text>
            </g>
          );
        })}
      </svg>
      <div className="legend static"><span>bar = fitted ÷ profile weight (log scale) · line = 95 % interval · grey = the layout says little about this weight · ⟂ = at the bound</span></div>
    </div>
  );
}

/** Status of the fit and the reference layouts ranked under the profile and under the fitted weights. */
export function FitSummary({ res, label }: { res: FitResponse; label: string }) {
  const f = res.fit;
  const status = f.self_consistent
    ? "Self-consistent: no better layout found under its fitted weights."
    : f.converged
      ? `Not optimal under any nearby weights: under its best weights a layout ${fmt(f.fitted.gap_pct, 2)} % better exists, ${f.fitted.keys} keys away.`
      : `Still improving after ${f.rounds.length} rounds (${fmt(f.fitted.gap_pct, 2)} % gap); a larger budget may close it.`;
  const rows = [...res.layouts].sort((a, b) => a.fitted - b.fitted);
  return (
    <>
      <div>
        <h4>Fit to {label}</h4>
        <p><b>Under this profile.</b> {f.base.gap > 0 ? `${fmt(f.base.gap_pct, 1)} % above the best layout found (${f.base.keys} keys away).` : "Nothing better found."}</p>
        <p><b>Under its fitted weights.</b> {status}</p>
        <p><b>Distance.</b> RMS log change {fmt(f.log_distance, 2)} (×{fmt(Math.exp(f.log_distance), 2)} per weight on average) after {f.rounds.length} rounds.</p>
        <p className="hint">The fit finds the smallest change of weights that makes the layout beat its swaps and every layout the search finds (a log-normal prior around the profile; weights normalized to a geometric mean of ×1). Apply the weights to score other layouts with this layout's preferences.</p>
      </div>
      <table className="tbl">
        <thead><tr><th>layout</th><th>rank (profile)</th><th>rank (fitted)</th><th>fitted score</th></tr></thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.layout}><td>{r.layout === "current" ? label : r.layout}</td><td>{r.rank_base}</td><td>{r.rank_fitted}</td><td>{fmt(r.fitted, 2)}</td></tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
