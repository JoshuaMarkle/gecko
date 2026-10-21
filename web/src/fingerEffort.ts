import { FINGERS, type Geometry, type Profile } from './api';

export interface FingerEffort {
  typing: number[];
  lateral: number[];
  vertical: number[];
  symmetric: boolean;
}
const mirror = (v: number[]) => [...v, ...v.slice().reverse()];
export function defaultFingerEffort(): FingerEffort {
  return {
    typing: mirror([2, 1.5, 1.1, 1, 1]),
    lateral: mirror([2.5, 1.8, 1.4, 0.7, 0.6]),
    vertical: mirror([1.5, 1.1, 0.9, 0.8, 0.8]),
    symmetric: true,
  };
}
export const withFingerEffort = (p: Profile): Profile => ({ ...p, finger_effort: p.finger_effort ?? defaultFingerEffort() });
export function fingerTargets(e: FingerEffort, g: Geometry): number[] {
  const present = new Set(g.key.map(k => k.finger));
  const capacity = FINGERS.map((f, i) => present.has(f) ? 1 / e.typing[e.symmetric && i >= 5 ? 9 - i : i] : 0);
  const total = capacity.reduce((a, b) => a + b, 0);
  return capacity.map(v => total ? 100 * v / total : 0);
}
export function editFingerEffort(e: FingerEffort, field: 'typing' | 'lateral' | 'vertical', i: number, value: number): FingerEffort {
  const values = [...e[field]];
  values[i] = Math.max(field === 'typing' ? 0.01 : 0, value);
  if (e.symmetric) values[9 - i] = values[i];
  return { ...e, [field]: values };
}
export function symmetricEffort(e: FingerEffort, symmetric: boolean): FingerEffort {
  return { ...e, symmetric, ...Object.fromEntries((['typing', 'lateral', 'vertical'] as const).map(k =>
    [k, e.symmetric || symmetric ? mirror(e[k].slice(0, 5)) : [...e[k]]])) };
}
