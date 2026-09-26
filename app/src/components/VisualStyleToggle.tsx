import { useEffect, useRef, useState, type ReactNode } from "react";
import { VISUAL_STYLES, type VisualStyle } from "../render/visualStyle";

// The 3D view's visual style toggle (ADR-038, design 1F): a pill in the lower-left corner
// showing the current style; hovering (or focusing) it opens all five, Wireframe to
// Realistic.

const INK = "#1b1d1f";
const T = "M12 3L21 8L12 13L3 8Z";
const L = "M3 8L12 13V22L3 17Z";
const R = "M21 8L12 13V22L21 17Z";

function Faces({ t, l, r, stroke = true }: { t: string; l: string; r: string; stroke?: boolean }) {
  const s = stroke ? { stroke: INK, strokeWidth: 1.4, strokeLinejoin: "round" as const } : {};
  return (
    <>
      <path d={T} fill={t} {...s} />
      <path d={L} fill={l} {...s} />
      <path d={R} fill={r} {...s} />
    </>
  );
}

export const STYLE_ICONS: Record<VisualStyle, ReactNode> = {
  wireframe: (
    <>
      <Faces t="none" l="none" r="none" />
      <path
        d="M3 17L12 12L21 17M12 3V12"
        fill="none"
        stroke={INK}
        strokeWidth="1.1"
        strokeDasharray="1.6 1.6"
      />
    </>
  ),
  hiddenLine: <Faces t="#fff" l="#fff" r="#fff" />,
  shaded: <Faces t="#dcdfe2" l="#a4a9ae" r="#747a81" />,
  consistent: <Faces t="#45484d" l="#b5a47f" r="#b5a47f" />,
  realistic: (
    <>
      <Faces t="#4a4e54" l="#c7b891" r="#8f805e" stroke={false} />
      <path
        d="M3 11L12 16M3 14L12 19M21 11L12 16M21 14L12 19"
        stroke="#6f6247"
        strokeWidth=".8"
        strokeOpacity=".6"
      />
    </>
  ),
};

export function StyleIcon({ style, size = 20 }: { style: VisualStyle; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden>
      {STYLE_ICONS[style]}
    </svg>
  );
}

export function VisualStyleToggle({
  value,
  onChange,
}: {
  value: VisualStyle;
  onChange: (s: VisualStyle) => void;
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
  const shown = open ? VISUAL_STYLES : VISUAL_STYLES.filter((s) => s.id === value);
  const move = (by: number) => {
    const i = VISUAL_STYLES.findIndex((s) => s.id === value);
    const next = VISUAL_STYLES[(i + by + VISUAL_STYLES.length) % VISUAL_STYLES.length]!;
    onChange(next.id);
    setOpen(true);
    requestAnimationFrame(() =>
      ref.current?.querySelector<HTMLButtonElement>(`[data-style="${next.id}"]`)?.focus(),
    );
  };
  return (
    <div
      ref={ref}
      className={`vstyle${open ? " open" : ""}`}
      role="radiogroup"
      aria-label="Visual Style"
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
      {shown.map((s) => (
        <button
          key={s.id}
          data-style={s.id}
          role="radio"
          aria-checked={s.id === value}
          aria-label={s.label}
          title={s.label}
          tabIndex={s.id === value ? 0 : -1}
          className={open && s.id === value ? "on" : undefined}
          onPointerDown={(e) => {
            // A tap on the closed pill opens it rather than choosing.
            if (!open && e.pointerType !== "mouse") {
              e.preventDefault();
              setOpen(true);
            }
          }}
          onClick={() => {
            if (open) onChange(s.id);
            else setOpen(true);
          }}
        >
          <StyleIcon style={s.id} />
        </button>
      ))}
    </div>
  );
}
