// The promo's 3D stage: a white studio. Light white fog, a white floor that fades into it, a soft floor
// shadow, and a light rig (key, cool fill, rim, lightformer environment for the cap sheen).
// Optional depth of field for the macro shots. Units: mm.

import { Environment, Lightformer, MeshReflectorMaterial } from "@react-three/drei";
import { useThree } from "@react-three/fiber";
import { DepthOfField, EffectComposer, Vignette } from "@react-three/postprocessing";
import { ThreeCanvas } from "@remotion/three";
import { useEffect, useLayoutEffect, type ReactNode } from "react";
import { continueRender, delayRender, useCurrentFrame, useVideoConfig } from "remotion";
import * as THREE from "three";
import { MAT } from "../lib/tokens";

export interface DofSpec {
  /** World point in focus (mm). */
  target: [number, number, number];
  /** Bokeh size; ~4–8 for macro. */
  bokeh?: number;
  /** In-focus depth range (world mm). */
  range?: number;
}

interface Props {
  children: ReactNode;
  /** Fog start / end (mm) relative to the camera target's distance: [−a, b] = clear until a in front of it, white b behind. */
  fog?: [number, number];
  /** Show the floor and its shadow. */
  floor?: boolean;
  /** Floor height (mm). */
  floorY?: number;
  dof?: DofSpec | null;
  /** Transparent background (to composite over 2D layers). */
  transparent?: boolean;
  /** Floor shadow opacity (the key light's soft shadow on a shadow catcher). */
  shadow?: number;
  /** Post effects (vignette, depth of field). */
  fx?: boolean;
  /** Studio environment map. */
  env?: boolean;
  /** Floor mirror strength (0 = matte floor). */
  reflect?: number;
  /** Vignette darkness. */
  vignette?: number;
}

function Fog({ near, far, transparent }: { near: number; far: number; transparent: boolean }) {
  const { scene, gl } = useThree();
  useLayoutEffect(() => {
    scene.fog = new THREE.Fog(MAT.fog, 400 + near, 400 + far);
    scene.userData.fogAround = [near, far];
    scene.background = transparent ? null : new THREE.Color(MAT.stage);
    gl.toneMapping = THREE.NeutralToneMapping;
    gl.toneMappingExposure = 1.0;
    gl.shadowMap.enabled = true;
    gl.shadowMap.type = THREE.VSMShadowMap;
  }, [scene, gl, near, far, transparent]);
  return null;
}

/**
 * ThreeCanvas advances R3F once per frame, before effect passes (EffectComposer) and environment
 * maps finish setting up on a fresh mount. Advance again on the next animation frame, holding the
 * render until it has drawn, so every frame (and every still) is complete.
 */
function Settle() {
  const frame = useCurrentFrame();
  const { advance } = useThree();
  useEffect(() => {
    const h = delayRender(`settle frame ${frame}`);
    let id = requestAnimationFrame(() => {
      advance(performance.now());
      id = requestAnimationFrame(() => { advance(performance.now()); continueRender(h); });
    });
    return () => { cancelAnimationFrame(id); continueRender(h); };
  }, [frame, advance]);
  return null;
}

export function Scene3D({ children, fog = [300, 3200], floor = true, floorY = 0, dof = null, transparent = false, shadow = 0.28, fx = true, env = true, reflect = 0, vignette = 0 }: Props) {
  const { width, height } = useVideoConfig();
  return (
    <ThreeCanvas width={width} height={height} gl={{ antialias: true, preserveDrawingBuffer: true, alpha: transparent }} shadows
      camera={{ fov: 30, near: 1, far: 8000, position: [0, 400, 600] }}>
      <Settle />
      <Fog near={fog[0]} far={fog[1]} transparent={transparent} />
      <hemisphereLight args={["#ffffff", "#d8dadf", 0.9]} />
      {/* key (casts the floor shadow), cool fill, rim from behind, front kicker */}
      <directionalLight position={[-160, 640, 220]} intensity={1.5} castShadow
        shadow-mapSize={[4096, 4096]} shadow-bias={-0.0004} shadow-normalBias={0.4} shadow-radius={14} shadow-blurSamples={24}
        shadow-camera-left={-320} shadow-camera-right={320} shadow-camera-top={320} shadow-camera-bottom={-320} shadow-camera-near={50} shadow-camera-far={1600} />
      <directionalLight position={[420, 220, 120]} intensity={0.35} color="#cdd9ff" />
      <directionalLight position={[60, 260, -640]} intensity={0.9} color="#dfe8ff" />
      <directionalLight position={[0, 90, 600]} intensity={0.3} />
      {env && <Environment resolution={256} frames={1}>
        <Lightformer form="rect" intensity={1.5} position={[0, 6, 2]} scale={[10, 4, 1]} rotation-x={Math.PI / 2.4} />
        <Lightformer form="rect" intensity={0.5} position={[-6, 2, 3]} scale={[4, 6, 1]} rotation-y={Math.PI / 3} />
        <Lightformer form="rect" intensity={0.7} position={[6, 2, -3]} scale={[4, 6, 1]} rotation-y={-Math.PI / 3} />
        <Lightformer form="ring" intensity={0.8} position={[0, 3, -8]} scale={4} />
      </Environment>}
      {floor && (
        <>
          <mesh rotation-x={-Math.PI / 2} position={[0, floorY - 3, 0]}>
            <planeGeometry args={[12000, 12000]} />
            {reflect > 0
              ? <MeshReflectorMaterial resolution={1024} mirror={reflect} blur={[420, 120]} mixBlur={1.2} mixStrength={2.2} depthScale={0.9}
                  minDepthThreshold={0.4} maxDepthThreshold={1.4} roughness={0.85} metalness={0.4} color={MAT.floor} />
              : <meshBasicMaterial color={MAT.floor} toneMapped={false} />}
          </mesh>
          {shadow > 0 && (
            <mesh rotation-x={-Math.PI / 2} position={[0, floorY + 0.05, 0]} receiveShadow>
              <planeGeometry args={[12000, 12000]} />
              <shadowMaterial opacity={shadow} color="#000000" transparent />
            </mesh>
          )}
        </>
      )}
      {children}
      {fx && <EffectComposer multisampling={4} enableNormalPass={false}>
        {dof ? <DepthOfField target={dof.target} worldFocusRange={dof.range ?? 40} bokehScale={dof.bokeh ?? 5} /> : <></>}
        <Vignette offset={0.3} darkness={vignette} />

      </EffectComposer>}
    </ThreeCanvas>
  );
}
