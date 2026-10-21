import { useId } from "react";
import { algoName } from "./SearchView";

/** The two prepared inputs meet at the scorer; search repeatedly evaluates layouts. */
export function OptimizationFlow({ algo }: { algo: string }) {
  const id = useId();
  return <section className="optimization-flow">
    <div className="optimization-flow-scroll">
      <svg viewBox="0 0 640 256" role="img" aria-labelledby={`${id}-title ${id}-desc`}>
        <title id={`${id}-title`}>How keyboard optimization works</title>
        <desc id={`${id}-desc`}>Language compiles to character, bigram, trigram and skipgram frequencies.
          The physical keyboard compiles to effort per key and movement and sequence costs.
          Both feed the scoring model, which combines frequency times physical cost with profile weights
          and balance penalties. Optimize uses {algoName(algo)} to search and score candidate layouts,
          then outputs a keyboard layout.</desc>
        <defs><marker id={`${id}-arrow`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M1 1 L7 4 L1 7" />
        </marker></defs>
        <g className="flow-links" markerEnd={`url(#${id}-arrow)`}>
          <path d="M202 65 H218 V108 H238" />
          <path d="M202 192 H218 V144 H238" />
          <path d="M412 126 H446" />
          <path d="M536 154 V192" />
        </g>
        <g className="flow-node">
          <rect x="12" y="16" width="190" height="98" rx="6" />
          <text x="24" y="38" className="flow-title">Language</text>
          <text x="24" y="59">Compiles frequencies</text>
          <text x="24" y="79">• Characters</text>
          <text x="24" y="98">• Bigrams · trigrams · skipgrams</text>
        </g>
        <g className="flow-node">
          <rect x="12" y="142" width="190" height="98" rx="6" />
          <text x="24" y="164" className="flow-title">Physical keyboard</text>
          <text x="24" y="185">Compiles physical costs</text>
          <text x="24" y="205">• Effort per key</text>
          <text x="24" y="224">• Movement · rolls · sequences</text>
        </g>
        <g className="flow-node flow-scoring">
          <rect x="238" y="67" width="174" height="118" rx="6" />
          <text x="250" y="89" className="flow-title">Scoring model</text>
          <text x="250" y="112">Frequency × physical cost</text>
          <text x="250" y="133">• Profile weights + balance</text>
          <text x="250" y="154">• One score per layout</text>
          <text x="250" y="173">• Lower is better</text>
        </g>
        <g className="flow-node">
          <rect x="446" y="78" width="182" height="76" rx="6" />
          <text x="458" y="100" className="flow-title">Optimize</text>
          <text x="458" y="121">• Algorithm</text>
          <text x="458" y="142">• Search & score candidates</text>
        </g>
        <g className="flow-node">
          <rect x="446" y="192" width="182" height="48" rx="6" />
          <text x="458" y="212" className="flow-title">Output</text>
          <text x="458" y="230">• Keyboard layout</text>
        </g>
      </svg>
    </div>
  </section>;
}
