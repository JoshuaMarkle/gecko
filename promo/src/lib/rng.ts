// Seeded RNG (mulberry32) so glitch, lift and scatter are the same on every render.

export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A stable random number for (seed, i), without carrying generator state. */
export const hash01 = (seed: number, i: number) => rng((seed * 2654435761) ^ (i * 40503))();
