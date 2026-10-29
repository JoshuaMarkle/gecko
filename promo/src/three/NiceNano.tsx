// nice!nano v2 on socket strips, plus the half's TRRS jack and reset button (mm).
// Origin: the PCB top surface under the nano's far (USB) end, centred; +z runs toward the typist.
// After ../kinetic/keyboard (nRF52840 with markings, castellated pins, crystal, regulator,
// passives, chip antenna, hollow USB-C shell), at a higher detail level.

import { useMemo } from "react";
import * as THREE from "three";
import { MAT } from "../lib/tokens";
import { roundBox } from "./geometry";
import { faceTexture } from "./legend";
import { matte, metal, topFace } from "./materials";

export const NANO = { w: 18, l: 33, t: 1.6, lift: 3 } as const;
/** Pin columns ±PIN_X from the centre; pin i at PIN_Z(i) from the far end (Pro Micro footprint). */
const PIN_X = 7.62;
const PIN_Z = (i: number) => 2.77 + i * 2.54;

const silk = () => faceTexture("nano-silk", NANO.w, NANO.l, "#0d0e12", (g) => {
  g.strokeStyle = g.fillStyle = MAT.silk;
  // Castellated half-holes along both long edges and their pad rings.
  for (let i = 0; i < 12; i++) for (const s of [-1, 1]) {
    g.fillStyle = MAT.copper;
    g.beginPath(); g.arc(s * PIN_X, PIN_Z(i), 0.85, 0, Math.PI * 2); g.fill();
    g.fillStyle = "#020203";
    g.beginPath(); g.arc(s * PIN_X, PIN_Z(i), 0.42, 0, Math.PI * 2); g.fill();
    g.fillStyle = MAT.copper;
    g.fillRect(s * (NANO.w / 2) - 0.6, PIN_Z(i) - 0.55, 1.2, 1.1);
  }
  g.fillStyle = MAT.silk;
  g.lineWidth = 0.12;
  // Pin labels (Pro Micro names), tiny.
  const L = ["D1", "D0", "GND", "GND", "D2", "D3", "D4", "C6", "D7", "E6", "B4", "B5"];
  const R = ["RAW", "GND", "RST", "VCC", "F4", "F5", "F6", "F7", "B1", "B3", "B2", "B6"];
  g.font = "0.62px JetBrains Mono, monospace";
  g.textBaseline = "middle";
  L.forEach((s, i) => { g.textAlign = "left"; g.fillText(s, -PIN_X + 1.15, PIN_Z(i)); });
  R.forEach((s, i) => { g.textAlign = "right"; g.fillText(s, PIN_X - 1.15, PIN_Z(i)); });
  // Fine traces between the pads and the module.
  g.lineWidth = 0.14;
  g.strokeStyle = "#20232b";
  for (let i = 1; i < 11; i++) for (const s of [-1, 1]) {
    g.beginPath(); g.moveTo(s * (PIN_X - 0.9), PIN_Z(i)); g.lineTo(s * 4.6, PIN_Z(i)); g.lineTo(s * 3.6, PIN_Z(i) - 1); g.stroke();
  }
  g.fillStyle = MAT.silk;
  g.textAlign = "center";
  g.font = "650 1.5px Inter, sans-serif";
  g.fillText("nice!nano", 0, 27.8);
  g.font = "0.7px JetBrains Mono, monospace";
  g.fillText("v2.0", 0, 29.2);
  g.lineWidth = 0.12; g.strokeStyle = MAT.silk;
  g.strokeRect(-2.4, 29.9, 4.8, 2.2); // antenna keep-out
});

/** Ink IC top with a pin-1 dot and etched marking lines. */
const chip = (id: string, w: number, d: number, mark: string[]) => faceTexture(`chip-${id}`, w, d, MAT.chip, (g) => {
  g.fillStyle = "#3a3a3a";
  g.beginPath(); g.arc(-w / 2 + 0.7, 0.7, 0.3, 0, Math.PI * 2); g.fill();
  g.fillStyle = "#8a8a8a";
  g.font = `500 ${w / 7.5}px JetBrains Mono, monospace`;
  g.textAlign = "center"; g.textBaseline = "middle";
  mark.forEach((s, i) => g.fillText(s, 0, d / 2 + (i - (mark.length - 1) / 2) * (w / 5.6)));
}, 80);

export function NiceNano() {
  const g = useMemo(() => {
    const usb = new THREE.Shape(), port = new THREE.Path();
    const pill = (sh: THREE.Shape | THREE.Path, w: number, h: number) => {
      const r = h / 2;
      sh.moveTo(-w / 2 + r, -r);
      sh.lineTo(w / 2 - r, -r); sh.absarc(w / 2 - r, 0, r, -Math.PI / 2, Math.PI / 2, false);
      sh.lineTo(-w / 2 + r, r); sh.absarc(-w / 2 + r, 0, r, Math.PI / 2, Math.PI * 1.5, false);
    };
    pill(usb, 8.94, 3.26);
    pill(port, 8.34, 2.66);
    usb.holes.push(port);
    return {
      board: new THREE.BoxGeometry(NANO.w, NANO.t, NANO.l),
      socket: roundBox(2.5, 12 * 2.54, NANO.lift, 0.3, 0.1),
      pin: new THREE.CylinderGeometry(0.32, 0.32, NANO.lift + NANO.t + 1.2, 8),
      solder: new THREE.SphereGeometry(0.75, 14, 8, 0, Math.PI * 2, 0, Math.PI / 2),
      usb: new THREE.ExtrudeGeometry(usb, { depth: 7.3, bevelEnabled: true, bevelThickness: 0.1, bevelSize: 0.08, bevelSegments: 1, curveSegments: 12 }),
      tongue: new THREE.BoxGeometry(6.6, 0.6, 5.2),
      box: new THREE.BoxGeometry(1, 1, 1),
    };
  }, []);
  const white = matte("#f7f7f5", 0.6), ink = matte(MAT.chip, 0.45), steel = metal(), gold = metal("#d4c08a", 0.3);
  const NT = NANO.lift + NANO.t;
  const at = (x: number, z: number, w: number, h: number, d: number, mat: THREE.Material | THREE.Material[], y = NT) => (
    <mesh geometry={g.box} scale={[w, h, d]} position={[x, y + h / 2, z]} material={mat} castShadow />
  );
  const passives: [number, number][] = [[-2, 8.4], [-0.7, 8.4], [0.7, 8.4], [2, 8.4], [-2.6, 15.8], [-1.2, 15.8], [1.2, 15.8], [-2.5, 23.6], [-1, 23.6], [0.6, 23.6], [2.2, 23.6], [3.4, 19.6], [-3.6, 13.4]];
  return (
    <group>
      {/* module PCB */}
      <mesh geometry={g.board} position={[0, NANO.lift + NANO.t / 2, NANO.l / 2]} material={topFace(silk(), matte("#16171c", 0.5))} castShadow receiveShadow />
      {/* socket strips, pins, solder fillets */}
      {[-1, 1].map((s) => (
        <group key={s} position={[s * PIN_X, 0, 0]}>
          <mesh geometry={g.socket} position={[0, 0, (PIN_Z(0) + PIN_Z(11)) / 2]} material={ink} castShadow />
          {Array.from({ length: 12 }, (_, i) => (
            <group key={i} position={[0, 0, PIN_Z(i)]}>
              <mesh geometry={g.pin} position={[0, (NANO.lift + NANO.t + 1.2) / 2, 0]} material={gold} />
              <mesh geometry={g.solder} position={[0, NT, 0]} scale={[1, 0.55, 1]} material={steel} />
            </group>
          ))}
        </group>
      ))}
      {/* nRF52840 (QIAA) with its 32 MHz crystal, the 32 kHz crystal, regulator, charger */}
      {at(0, 12, 7, 0.85, 7, topFace(chip("nrf", 7, 7, ["N52840", "QIAAD0", "2142AB"]), ink, 0.4))}
      {at(-2, 17.2, 2, 0.6, 1.6, steel)}
      {at(2, 17.2, 3.2, 0.8, 1.5, steel)}
      {at(1.8, 21, 3, 0.8, 3, topFace(chip("chg", 3, 3, ["BQ24"]), ink, 0.4))}
      {at(-1.8, 21, 1.6, 1.1, 2.9, topFace(chip("reg", 1.6, 2.9, ["L5"]), ink, 0.4))}
      {passives.map(([x, z], i) => <group key={i}>
        {at(x, z, 1, 0.5, 0.5, white)}
        {at(x - 0.42, z, 0.18, 0.52, 0.52, steel)}
        {at(x + 0.42, z, 0.18, 0.52, 0.52, steel)}
      </group>)}
      {[-1, 1].map((s) => <group key={s}>
        {at(s * 2.5, 25.4, 0.8, 0.45, 1.6, white)}
        {at(s * 2.5, 25.4, 0.5, 0.25, 0.9, matte("#e8e8e8", 0.2), NT + 0.45)}
      </group>)}
      {/* chip antenna */}
      {at(0, 30.8, 3.2, 1.1, 1.6, topFace(chip("ant", 3.2, 1.6, ["2.4G"]), white, 0.4))}
      {/* USB-C: hollow shell overhanging the far edge, tongue, legs */}
      <mesh geometry={g.usb} position={[0, NT + 1.63, -1]} material={steel} castShadow />
      <mesh geometry={g.tongue} position={[0, NT + 1.63, 3.4]} material={ink} />
      {at(0, 6.2, 8.3, 2.6, 0.3, steel, NT + 0.33)}
      {[-1, 1].map((s) => <group key={s}>{at(s * 4.7, 4.6, 0.5, 1.2, 1.2, steel)}</group>)}
    </group>
  );
}

/** TRRS jack: body along x with the socket barrel at its +x end. Origin on the PCB top. */
export function TRRS({ flip = false }: { flip?: boolean }) {
  const g = useMemo(() => ({
    body: roundBox(12, 6, 5, 0.6, 0.2),
    barrel: new THREE.CylinderGeometry(2.6, 2.6, 2.2, 32).rotateZ(Math.PI / 2),
    hole: new THREE.CylinderGeometry(1.75, 1.75, 0.3, 24).rotateZ(Math.PI / 2),
  }), []);
  const s = flip ? -1 : 1;
  return (
    <group scale={[s, 1, 1]}>
      <mesh geometry={g.body} material={matte(MAT.chip, 0.5)} castShadow />
      <mesh geometry={g.barrel} position={[7, 2.6, 0]} material={metal()} castShadow />
      <mesh geometry={g.hole} position={[8.15, 2.6, 0]} material={matte("#050505", 0.9)} />
    </group>
  );
}

/** Tactile reset button: square body, round plunger. Origin on the PCB top. */
export function ResetButton() {
  const g = useMemo(() => ({ body: roundBox(6, 6, 3.5, 0.4, 0.15), cap: new THREE.CylinderGeometry(1.7, 1.7, 1.2, 28) }), []);
  return (
    <group>
      <mesh geometry={g.body} material={metal("#bdbdbd", 0.4)} castShadow />
      <mesh geometry={g.cap} position={[0, 4.1, 0]} material={matte(MAT.chip, 0.5)} castShadow />
    </group>
  );
}
