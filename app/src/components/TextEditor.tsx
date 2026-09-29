import { useEffect, useRef, useState } from "react";
import type { TextAlign } from "../bindings/TextAlign";

// Revit's in-place text editor (ADR-070): a box on the canvas where the note goes, in the
// note's own font and size at the view's zoom. Enter starts a new line; clicking outside or
// Esc finishes (an empty note is dropped), as in Revit.

export function TextEditor({
  x,
  y,
  fontPx,
  widthPx,
  align,
  initial,
  onDone,
}: {
  /** Top-left of the note's box, screen px. */
  x: number;
  y: number;
  fontPx: number;
  widthPx: number | null;
  align: TextAlign;
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
      rows={lines.length}
      style={{
        left: x,
        top: y,
        fontSize: px,
        lineHeight: 1.6,
        textAlign: align === "Left" ? "left" : align === "Right" ? "right" : "center",
        width: widthPx ?? `${longest * 0.52 + 0.8}em`,
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
