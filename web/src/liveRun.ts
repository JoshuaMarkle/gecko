// The run started from the navbar: starts it on the server, follows its progress events (SSE),
// pauses / resumes / stops it, and reports how much of its budget it has used. Pause
// bookkeeping mirrors the server: paused time does not use up the budget.

import { useEffect, useRef, useState } from "react";
import { api, type Progress, type RunRequest } from "./api";

export interface Live {
  id: string; algo: string; progress: Progress[]; running: boolean;
  started: number; budgetMs: number | null; budgetEvals: number | null; pausedAt: number | null; pausedMs: number;
}

export interface LiveRunEvents {
  /** The run finished (or was stopped) with results saved. */
  onDone: (id: string, algo: string) => void;
  onError: (message: string) => void;
  /** The server's run list changed (a run started or ended). */
  onRunsChanged: () => void;
}

export function useLiveRun(events: LiveRunEvents) {
  const [live, setLive] = useState<Live | null>(null);
  const esRef = useRef<EventSource | null>(null);
  // Events read through a ref so a long run reports to the latest handlers.
  const on = useRef(events);
  on.current = events;

  useEffect(() => () => esRef.current?.close(), []);

  const start = async (req: RunRequest) => {
    const byEvals = req.evals !== undefined;
    const r = await api.startRun(req);
    setLive({
      id: r.id, algo: req.algo, progress: [], running: true, started: Date.now(),
      budgetMs: byEvals ? null : req.time_ms ?? null, budgetEvals: byEvals ? req.evals! : null, pausedAt: null, pausedMs: 0,
    });
    on.current.onRunsChanged();
    esRef.current?.close();
    const es = new EventSource(`/api/runs/${encodeURIComponent(r.id)}/events`);
    esRef.current = es;
    const end = () => { es.close(); setLive((l) => (l ? { ...l, running: false } : l)); };
    es.addEventListener("progress", (e) => {
      const p = JSON.parse((e as MessageEvent).data) as Progress;
      setLive((l) => (l && l.id === r.id ? { ...l, progress: [...l.progress, p] } : l));
    });
    es.addEventListener("done", () => { end(); on.current.onDone(r.id, req.algo); on.current.onRunsChanged(); });
    es.addEventListener("failed", (e) => { end(); on.current.onError(JSON.parse((e as MessageEvent).data).error); on.current.onRunsChanged(); });
    es.onerror = () => { if (es.readyState === EventSource.CLOSED) setLive((l) => (l ? { ...l, running: false } : l)); };
  };

  const stop = () => { if (live) api.cancel(live.id).catch(() => {}); };

  const paused = !!live?.running && live.pausedAt !== null;
  const setPaused = (pause: boolean) => {
    if (!live?.running || paused === pause) return;
    api.pause(live.id, pause).then(() => setLive((l) => {
      if (!l || l.id !== live.id) return l;
      return pause ? { ...l, pausedAt: Date.now() } : { ...l, pausedAt: null, pausedMs: l.pausedMs + Date.now() - (l.pausedAt ?? Date.now()) };
    })).catch((e) => on.current.onError((e as Error).message));
  };

  // Re-render while running so elapsed time and the budget fraction advance.
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!live?.running) return;
    const t = setInterval(() => setNow(Date.now()), 200);
    return () => clearInterval(t);
  }, [live?.running]);

  const last = live?.progress[live.progress.length - 1];
  const activeMs = live ? (live.pausedAt ?? now) - live.started - live.pausedMs : 0;
  /** Share of the budget used (1 once finished), or null with no run. */
  const frac = !live ? null : !live.running ? 1
    : live.budgetEvals ? Math.min(1, (last?.evals ?? 0) / live.budgetEvals) : Math.min(1, activeMs / (live.budgetMs ?? 1));

  return { live, last, paused, activeMs, frac, start, stop, setPaused };
}
