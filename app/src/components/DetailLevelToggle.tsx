import { useEffect, useRef, useState, type ReactNode } from "react";
import type { DetailLevel } from "../bindings/DetailLevel";

// The drawn views' Detail Level toggle (ADR-067, icon set 2C "Wall Layers"): the same pill
// as the 3D view's visual style toggle, in the lower-left corner, showing the current
// level; hovering (or focusing) it opens Coarse, Medium and Fine.

const INK = "#1b1d1f";

export const DETAIL_LEVELS: { id: DetailLevel; label: string; keys: string }[] = [
  { id: "Coarse", label: "Coarse", keys: "DC" },
  { id: "Medium", label: "Medium", keys: "DD" },
  { id: "Fine", label: "Fine", keys: "DF" },
];

const SLAB = { x: 3, y: 7, width: 18, height: 10, stroke: INK, strokeWidth: 1.4 };

/** A wall in plan: solid poché, its core boundaries, then every layer with insulation. */
export const DETAIL_ICONS: Record<DetailLevel, ReactNode> = {
  Coarse: <rect {...SLAB} fill={INK} />,
  Medium: (
    <>
      <rect {...SLAB} fill="#fff" />
      <path d="M3 9.5H21M3 14.5H21" stroke={INK} strokeWidth=".9" />
    </>
  ),
  Fine: (
    <>
      <rect {...SLAB} fill="#fff" />
      <path d="M3 8.8H21M3 10H21M3 15H21" stroke={INK} strokeWidth=".7" />
      <path
        d="M3 10L5 15L7 10L9 15L11 10L13 15L15 10L17 15L19 10L21 15"
        fill="none"
        stroke={INK}
        strokeWidth=".7"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </>
  ),
};

export function DetailIcon({ level, size = 20 }: { level: DetailLevel; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden>
      {DETAIL_ICONS[level]}
    </svg>
  );
}

export function DetailLevelToggle({
  value,
  onChange,
}: {
  value: DetailLevel;
  onChange: (d: DetailLevel) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  // Touch: a tap opens it; a tap outside closes it.
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", away);
    return () => window.removeEventListener("pointerdown", away);
  }, [open]);
  const shown = open ? DETAIL_LEVELS : DETAIL_LEVELS.filter((d) => d.id === value);
  const move = (by: number) => {
    const i = DETAIL_LEVELS.findIndex((d) => d.id === value);
    const next = DETAIL_LEVELS[Math.min(DETAIL_LEVELS.length - 1, Math.max(0, i + by))]!;
    onChange(next.id);
    setOpen(true);
    requestAnimationFrame(() =>
      ref.current?.querySelector<HTMLButtonElement>(`[data-level="${next.id}"]`)?.focus(),
    );
  };
  return (
    <div
      ref={ref}
      className={`vstyle dlevel${open ? " open" : ""}`}
      role="radiogroup"
      aria-label="Detail Level"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOpen(false);
      }}
      onKeyDown={(e) => {
        if (e.key === "ArrowRight") move(1);
        else if (e.key === "ArrowLeft") move(-1);
        else return;
        e.preventDefault();
        e.stopPropagation();
      }}
    >
      {shown.map((d) => (
        <button
          key={d.id}
          data-level={d.id}
          role="radio"
          aria-checked={d.id === value}
          aria-label={d.label}
          title={`${d.label} (${d.keys})`}
          tabIndex={d.id === value ? 0 : -1}
          className={open && d.id === value ? "on" : undefined}
          onPointerDown={(e) => {
            // A tap on the closed pill opens it rather than choosing.
            if (!open && e.pointerType !== "mouse") {
              e.preventDefault();
              setOpen(true);
            }
          }}
          onClick={() => {
            if (open) onChange(d.id);
            else setOpen(true);
          }}
        >
          <DetailIcon level={d.id} />
        </button>
      ))}
    </div>
  );
}
