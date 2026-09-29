import type { DisplayList } from "../bindings/DisplayList";

// A drafting view's display list drawn as SVG (ADR-069): the Detail Library's thumbnails and
// its larger preview. Pen weights follow the plan canvas's; small thumbnails leave out text.

const FILL: Record<string, string> = {
  Ink: "#1b1d1f",
  Poche: "#1b1d1f",
  PocheLight: "#b9bdc1",
  Paper: "#ffffff",
  Glass: "#dcebf2",
};

export function DetailThumb({
  dl,
  label,
  text = false,
}: {
  dl: DisplayList;
  label: string;
  text?: boolean;
}) {
  const [x0, y0, x1, y1] = dl.bounds;
  const w = x1 - x0;
  const h = y1 - y0;
  // Pen widths in paper mm (at the view's scale), so thumbnails read like the sheet.
  const pen = (n: number) => (0.13 + (n - 1) * 0.09) * dl.scale;
  const pt = (p: [number, number]) => `${(p[0] - x0).toFixed(1)},${(y1 - p[1]).toFixed(1)}`;
  return (
    <svg
      className="dl-thumb"
      viewBox={`0 0 ${w} ${h}`}
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label={label}
    >
      <rect x={0} y={0} width={w} height={h} fill="#fff" />
      {dl.items.map((it, i) => {
        const p = it.prim;
        if (p.t === "Fill") {
          const d = p.rings.map((r) => `M${r.map(pt).join("L")}Z`).join("");
          return <path key={i} d={d} fill={FILL[p.fill] ?? "#eee"} fillRule="evenodd" />;
        }
        if (p.t === "Line") {
          const l = p;
          return (
            <path
              key={i}
              d={`M${l.pts.map(pt).join("L")}${l.closed ? "Z" : ""}`}
              fill="none"
              stroke="#1b1d1f"
              strokeWidth={pen(l.w)}
              strokeDasharray={l.dash === "Solid" ? undefined : `${dl.scale * 2} ${dl.scale}`}
              strokeLinejoin="round"
            />
          );
        }
        if (p.t === "Circle") {
          const c = p;
          return (
            <circle
              key={i}
              cx={c.c[0] - x0}
              cy={y1 - c.c[1]}
              r={c.r}
              fill={c.filled ? "#1b1d1f" : "none"}
              stroke="#1b1d1f"
              strokeWidth={pen(1)}
            />
          );
        }
        if (p.t === "Text" && text) {
          const t = p;
          return (
            <text
              key={i}
              x={t.at[0] - x0}
              y={y1 - t.at[1]}
              fontSize={t.size}
              fontFamily="Arial, sans-serif"
              fill="#1b1d1f"
            >
              {t.text}
            </text>
          );
        }
        return null;
      })}
    </svg>
  );
}
