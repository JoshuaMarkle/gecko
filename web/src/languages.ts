// Languages (frontend.md "Language"): the bundled corpora (built, read-only) plus languages
// created in this browser. A local language's sources live in IndexedDB; Summarize builds its
// `.gkc` statistics in WASM (`corpus_from_text`). Local languages can be viewed but not yet
// optimized on: runs need the corpus on the server (an upload route is planned).

import { useCallback, useEffect, useState } from "react";
import { api, type CorpusInfo } from "./api";
import { usePref } from "./inputs";
import { corpusFromText, corpusStats, type CorpusStats } from "./wasm";

export interface Source { name: string; bytes: number; added: number }
export interface LocalLanguage { id: string; name: string; sources: Source[]; summarizedAt: number | null; words: number | null }
export type Status = "Summarized" | "Unsummarized" | "Outdated";

export interface Language {
  id: string;
  name: string;
  /** Bundled corpus (served by `gecko serve`) or created in this browser. */
  local: boolean;
  status: Status;
  /** Known sources; bundled sources load on demand (`null` until then). */
  sources: Source[] | null;
  chars: number | null;
  words: number | null;
}

export function statusOf(l: LocalLanguage): Status {
  if (l.summarizedAt === null) return "Unsummarized";
  return l.sources.some((s) => s.added > l.summarizedAt!) ? "Outdated" : "Summarized";
}

// ---------- IndexedDB (one object store of blobs keyed by path) ----------

function db(): Promise<IDBDatabase> {
  return new Promise((ok, fail) => {
    const r = indexedDB.open("gecko", 1);
    r.onupgradeneeded = () => r.result.createObjectStore("blobs");
    r.onsuccess = () => ok(r.result);
    r.onerror = () => fail(r.error);
  });
}

async function idb<T>(mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest): Promise<T> {
  const d = await db();
  return new Promise((ok, fail) => {
    const r = f(d.transaction("blobs", mode).objectStore("blobs"));
    r.onsuccess = () => ok(r.result as T);
    r.onerror = () => fail(r.error);
  });
}

const put = (key: string, v: unknown) => idb<void>("readwrite", (s) => s.put(v, key));
const get = <T>(key: string) => idb<T | undefined>("readonly", (s) => s.get(key));
const del = (range: IDBKeyRange) => idb<void>("readwrite", (s) => s.delete(range));

// ---------- hook ----------

export function useLanguages(corpora: CorpusInfo[]) {
  const [locals, setLocals] = usePref<LocalLanguage[]>("languages", []);
  const [selId, setSelId] = usePref<string>("language", "");
  const [bundledSources, setBundledSources] = useState<Record<string, Source[]>>({});
  const [busy, setBusy] = useState<string | null>(null);

  const languages: Language[] = [
    ...corpora.map((c): Language => ({ id: c.id, name: c.name, local: false, status: "Summarized", sources: bundledSources[c.id] ?? null, chars: c.chars, words: null })),
    ...locals.map((l): Language => ({ id: `local:${l.id}`, name: l.name, local: true, status: statusOf(l), sources: l.sources, chars: null, words: l.words })),
  ];
  const selected = languages.find((l) => l.id === selId) ?? languages[0];

  const loadSources = useCallback(async (id: string) => {
    if (id.startsWith("local:") || bundledSources[id]) return;
    try {
      const s = await (await fetch(`/api/corpora/${encodeURIComponent(id)}/sources`)).json();
      setBundledSources((b) => ({ ...b, [id]: Array.isArray(s) ? s.map((x: Source) => ({ ...x, added: 0 })) : [] }));
    } catch {
      setBundledSources((b) => ({ ...b, [id]: [] }));
    }
  }, [bundledSources]);

  const sourceText = useCallback(async (langId: string, name: string): Promise<string> => {
    if (langId.startsWith("local:")) return (await get<string>(`lang/${langId.slice(6)}/src/${name}`)) ?? "";
    const r = await fetch(`/api/corpora/${encodeURIComponent(langId)}/sources/${encodeURIComponent(name)}`);
    if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    return r.text();
  }, []);

  const create = (name: string) => {
    const id = `${name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "language"}-${Date.now().toString(36)}`;
    setLocals([...locals, { id, name, sources: [], summarizedAt: null, words: null }]);
    setSelId(`local:${id}`);
  };

  /** Add text files to a local language (replacing same-named sources). */
  const upload = async (langId: string, files: File[]) => {
    const id = langId.slice(6);
    const l = locals.find((x) => x.id === id);
    if (!l) return;
    const now = Date.now();
    const added: Source[] = [];
    for (const f of files) {
      await put(`lang/${id}/src/${f.name}`, await f.text());
      added.push({ name: f.name, bytes: f.size, added: now });
    }
    const names = new Set(added.map((s) => s.name));
    setLocals(locals.map((x) => (x.id === id ? { ...x, sources: [...x.sources.filter((s) => !names.has(s.name)), ...added] } : x)));
  };

  const remove = async (langId: string) => {
    const id = langId.slice(6);
    await del(IDBKeyRange.bound(`lang/${id}/`, `lang/${id}/￿`)).catch(() => {});
    setLocals(locals.filter((x) => x.id !== id));
    if (selId === langId) setSelId("");
  };

  /** Build a local language's statistics from all of its sources. */
  const summarize = async (langId: string) => {
    const id = langId.slice(6);
    const l = locals.find((x) => x.id === id);
    if (!l || !l.sources.length) return;
    setBusy(langId);
    try {
      const texts = await Promise.all(l.sources.map((s) => get<string>(`lang/${id}/src/${s.name}`)));
      const text = texts.filter(Boolean).join("\n\n");
      const bytes = await corpusFromText(l.name, "default", text);
      await put(`lang/${id}/gkc`, bytes);
      const words = (text.match(/\S+/g) ?? []).length;
      setLocals(locals.map((x) => (x.id === id ? { ...x, summarizedAt: Date.now(), words } : x)));
    } finally {
      setBusy(null);
    }
  };

  return { languages, selected, select: setSelId, loadSources, sourceText, create, upload, remove, summarize, busy };
}

export type Languages = ReturnType<typeof useLanguages>;

/** Statistics of a language: bundled from the served `.gkc`, local from the one Summarize built. */
export function useLanguageStats(lang: Language | undefined): { stats: CorpusStats | null; error: string | null } {
  const [stats, setStats] = useState<CorpusStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setStats(null);
    setError(null);
    if (!lang || lang.status === "Unsummarized") return;
    (async () => {
      try {
        const bytes = lang.local ? await get<Uint8Array>(`lang/${lang.id.slice(6)}/gkc`) : await api.corpusBytes(lang.id);
        if (!bytes) return;
        const s = await corpusStats(bytes, 20);
        if (alive) setStats(s);
      } catch (e) {
        if (alive) setError((e as Error).message);
      }
    })();
    return () => { alive = false; };
  }, [lang?.id, lang?.status, lang?.words]); // eslint-disable-line react-hooks/exhaustive-deps
  return { stats, error };
}
