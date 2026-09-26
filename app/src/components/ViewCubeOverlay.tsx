import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import {
  CUBE_GAP,
  CUBE_SIZE,
  saveHome,
  savedHome,
  spotDir,
  spotName,
  type Aligned,
  type Spot,
  type ViewCube,
} from "../render/viewCube";

// The ViewCube's controls (ADR-037): the corner that takes the cube's pointer input, the
// Home button, the arrows to the next face when looking straight at one, and the menu.

const ARROWS: { key: keyof Omit<Aligned, "face">; x: number; y: number; rot: number }[] = [
  { key: "up", x: 0, y: -1, rot: 0 },
  { key: "right", x: 1, y: 0, rot: 90 },
  { key: "down", x: 0, y: 1, rot: 180 },
  { key: "left", x: -1, y: 0, rot: 270 },
];

export function ViewCubeOverlay({ cube, viewId }: { cube: ViewCube | null; viewId: string }) {
  const hit = useRef<HTMLDivElement>(null);
  const [aligned, setAligned] = useState<Aligned | null>(null);
  const [tip, setTip] = useState("");
  const [menu, setMenu] = useState(false);
  // Bumped when home is set or reset, to show the menu right.
  const [, setHomeRev] = useState(0);
  const hasHome = !!savedHome(viewId);

  useEffect(() => {
    if (!cube || !hit.current) return;
    const unwatch = cube.watch(setAligned, setTip);
    const off = cube.attach(hit.current);
    return () => {
      off();
      unwatch();
    };
  }, [cube]);
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(false);
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  }, [menu]);

  const act = (f: () => void) => () => {
    setMenu(false);
    f();
  };
  const setHome = () => {
    if (!cube) return;
    saveHome(viewId, cube.host.camera.position.clone(), cube.host.target.clone());
    setHomeRev((n) => n + 1);
  };
  const resetHome = () => {
    saveHome(viewId, null);
    setHomeRev((n) => n + 1);
  };
  const go = (s: Spot) => cube?.orient(spotDir(s));
  // The arrows sit just outside the face as seen straight on.
  const face = CUBE_SIZE / 4.5;
  return (
    <div
      className="viewcube"
      style={{ top: CUBE_GAP, right: CUBE_GAP, width: CUBE_SIZE, height: CUBE_SIZE }}
      role="group"
      aria-label="ViewCube"
    >
      <div
        ref={hit}
        className="viewcube-hit"
        title={tip}
        onContextMenu={(e) => {
          e.preventDefault();
          setMenu(true);
        }}
      />
      <button
        className="viewcube-home"
        aria-label="Home"
        title="Home"
        onClick={() => cube?.goHome()}
      >
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden>
          <path d="M2 8.2 8 2.8l6 5.4" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M4 7.2V13h3v-3.2h2V13h3V7.2" fill="currentColor" />
        </svg>
      </button>
      {aligned &&
        ARROWS.map((a) => (
          <button
            key={a.key}
            className="viewcube-arrow"
            aria-label={`Turn to ${spotName(aligned[a.key])}`}
            title={spotName(aligned[a.key])}
            style={{
              left: CUBE_SIZE / 2 + a.x * (face + 11) - 7,
              top: CUBE_SIZE / 2 + a.y * (face + 11) - 7,
              transform: `rotate(${a.rot}deg)`,
            }}
            onClick={() => go(aligned[a.key])}
          >
            <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden>
              <path d="M7 2.5 12 10H2z" fill="currentColor" />
            </svg>
          </button>
        ))}
      <button
        className="viewcube-menu-btn"
        aria-label="ViewCube menu"
        title="ViewCube options"
        onPointerDown={(e) => e.stopPropagation()}
        onClick={() => setMenu((m) => !m)}
      >
        <svg viewBox="0 0 10 10" width="9" height="9" aria-hidden>
          <path d="M1 3h8L5 8z" fill="currentColor" />
        </svg>
      </button>
      {menu && (
        <div className="viewcube-menu" role="menu" onPointerDown={(e) => e.stopPropagation()}>
          <button role="menuitem" onClick={act(() => cube?.goHome())}>
            Go Home
          </button>
          <button role="menuitem" onClick={act(setHome)}>
            Set Current View as Home
          </button>
          <button role="menuitem" disabled={!hasHome} onClick={act(resetHome)}>
            Reset Home
          </button>
          <hr />
          <button role="menuitem" onClick={act(() => cube?.fit())}>
            Fit to View
          </button>
          <button role="menuitem" onClick={act(() => go([0, 0, 1]))}>
            Orient to Top
          </button>
          <button role="menuitem" onClick={act(() => cube?.orient(new THREE.Vector3(1, -1, 1)))}>
            Orient to Southeast
          </button>
        </div>
      )}
    </div>
  );
}
