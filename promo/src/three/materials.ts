// Shared 3D materials, cached so 42 keys don't make 42 copies. Flat matte white plastics;
// colour comes only from the keyboard data (finger fills) and the one accent.

import * as THREE from "three";
import { MAT } from "../lib/tokens";

const cache = new Map<string, THREE.Material>();
function get<T extends THREE.Material>(id: string, make: () => T): T {
  let m = cache.get(id) as T | undefined;
  if (!m) { m = make(); cache.set(id, m); }
  return m;
}

/** PBT-like keycap plastic: matte, no clearcoat or sheen (the caps read as dry, textured plastic). */
export const plastic = (color: string, opacity = 1) => get(`plastic|${color}|${opacity}`, () => new THREE.MeshPhysicalMaterial({
  color, roughness: 0.92, metalness: 0, specularIntensity: 0.35, transparent: opacity < 1, opacity,
}));

/** Keycap top: the legend texture carries the colour. */
export const legendMat = (map: THREE.Texture, opacity = 1) => get(`legend|${map.uuid}|${opacity}`, () => new THREE.MeshPhysicalMaterial({
  map, roughness: 0.9, metalness: 0, specularIntensity: 0.35, transparent: opacity < 1, opacity,
}));

export const matte = (color: string, roughness = 0.8, opacity = 1) => get(`matte|${color}|${roughness}|${opacity}`, () =>
  new THREE.MeshStandardMaterial({ color, roughness, metalness: 0, transparent: opacity < 1, opacity }));

export const metal = (color: string = MAT.metal, roughness = 0.32) => get(`metal|${color}|${roughness}`, () =>
  new THREE.MeshStandardMaterial({ color, roughness, metalness: 0.9 }));

/** Anodized aluminium switch plate: dark, matte (no clearcoat or metal sheen). */
export const plateMat = (opacity = 1) => get(`plate|${opacity}`, () => new THREE.MeshPhysicalMaterial({
  color: MAT.plate, roughness: 0.9, metalness: 0, specularIntensity: 0.3, transparent: opacity < 1, opacity,
}));

/** Translucent housing top (clear-top switch look, kept white). */
export const housingTop = () => get("housingTop", () => new THREE.MeshPhysicalMaterial({
  color: MAT.housing, roughness: 0.35, metalness: 0, transmission: 0.25, thickness: 1.5, clearcoat: 0.4,
}));

/** A face texture over the top of a box, plain material on the other five faces. */
export const topFace = (map: THREE.Texture, side: THREE.Material, roughness = 0.6) =>
  [side, side, get(`face|${map.uuid}|${roughness}`, () => new THREE.MeshStandardMaterial({ map, roughness, metalness: 0 })), side, side, side];
