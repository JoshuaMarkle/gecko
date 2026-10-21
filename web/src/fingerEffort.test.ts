import { describe, expect, it } from 'vitest';
import { defaultFingerEffort, editFingerEffort, fingerTargets, symmetricEffort } from './fingerEffort';
import type { Geometry } from './api';

describe('relative finger effort', () => {
  it('normalizes inverse effort over present fingers, including thumbs', () => {
    const g = { key: [{ finger: 'LP' }, { finger: 'LI' }, { finger: 'LT' }] } as Geometry;
    const t = fingerTargets(defaultFingerEffort(), g);
    expect(t).toEqual([20, 0, 0, 40, 40, 0, 0, 0, 0, 0]);
  });
  it('mirrors edits and allows independent costs after unlocking', () => {
    let e = editFingerEffort(defaultFingerEffort(), 'typing', 0, 3);
    expect(e.typing[9]).toBe(3);
    e = editFingerEffort(symmetricEffort(e, false), 'typing', 9, 4);
    expect(e.typing[0]).toBe(3);
    expect(e.typing[9]).toBe(4);
    expect(symmetricEffort(e, true).typing[9]).toBe(3);
    expect(editFingerEffort(e, 'typing', 0, 0).typing[0]).toBeGreaterThan(0);
  });
});
