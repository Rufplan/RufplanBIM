import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { TextAlign } from "../bindings/TextAlign";

// Revit's in-place text editor (ADR-070, ADR-108): a box exactly where the note is, in its
// own font and size at the view's zoom, its wrap width and its turn. Lines wrap as you type
// and the box grows with them. Enter starts a new line; clicking outside or Esc finishes
// (an empty note is dropped), as in Revit.

export function TextEditor({
  x,
  y,
  fontPx,
  widthPx,
  align,
  angle = 0,
  origin = [0, 0],
  initial,
  onDone,
}: {
  /** Top-left of the note's box, screen px. */
  x: number;
  y: number;
  fontPx: number;
  widthPx: number | null;
  align: TextAlign;
  /** The note's turn, radians counter-clockwise, about `origin` (px from the box's top-left). */
  angle?: number;
  origin?: [number, number];
  initial: string;
  onDone: (text: string | null) => void;
}) {
  const [text, setText] = useState(initial);
  const ref = useRef<HTMLTextAreaElement>(null);
  const done = useRef(false);
  useEffect(() => {
    const t = ref.current;
    if (!t) return;
    t.focus();
    t.setSelectionRange(t.value.length, t.value.length);
  }, []);
  // The box grows with the wrapped lines, not just the typed ones.
  useLayoutEffect(() => {
    const t = ref.current;
    if (!t) return;
    t.style.height = "auto";
    t.style.height = `${t.scrollHeight}px`;
  }, [text, widthPx, fontPx]);
  const finish = (value: string | null) => {
    if (done.current) return;
    done.current = true;
    onDone(value);
  };
  const lines = text.split("\n");
  const longest = Math.max(4, ...lines.map((l) => l.length + 1));
  const px = Math.max(9, fontPx);
  return (
    <textarea
      ref={ref}
      className="text-editor"
      aria-label="Text"
      spellCheck
      value={text}
      rows={1}
      style={{
        left: x,
        top: y,
        fontSize: px,
        lineHeight: 1.6,
        textAlign: align === "Left" ? "left" : align === "Right" ? "right" : "center",
        width: widthPx ?? `${longest * 0.52 + 0.8}em`,
        // Canvas y points down, so the model's counter-clockwise turn is negative here.
        transform: angle ? `rotate(${-angle}rad)` : undefined,
        transformOrigin: `${origin[0]}px ${origin[1]}px`,
      }}
      onChange={(e) => setText(e.target.value)}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Escape") {
          e.preventDefault();
          finish(text);
        } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
          e.preventDefault();
          finish(text);
        }
      }}
      onBlur={() => finish(text)}
      onMouseDown={(e) => e.stopPropagation()}
    />
  );
}
