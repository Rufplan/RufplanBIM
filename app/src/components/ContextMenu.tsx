import { useEffect, useRef, useState } from "react";

// A right-click menu (ADR-074), styled like the ribbon's menus: items, separators and
// submenus that open on hover (Revit's Duplicate View ▸). Esc or a click away closes it.

export interface MenuItem {
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  items?: MenuItem[];
  /** A separator above this item. */
  separator?: boolean;
}

function Items({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  const [open, setOpen] = useState<number | null>(null);
  return (
    <>
      {items.map((it, i) => (
        <div
          key={`${it.label}-${i}`}
          className={`ctx-row${it.separator ? " sep" : ""}`}
          onMouseEnter={() => setOpen(it.items ? i : null)}
        >
          <button
            role="menuitem"
            className="ctx-item"
            disabled={it.disabled}
            aria-haspopup={it.items ? "menu" : undefined}
            aria-expanded={it.items ? open === i : undefined}
            onClick={() => {
              if (it.items) {
                setOpen(i);
                return;
              }
              onClose();
              it.onClick?.();
            }}
          >
            <span>{it.label}</span>
            {it.items && <span className="ctx-caret">▸</span>}
          </button>
          {it.items && open === i && (
            <div className="ctx-menu ctx-sub" role="menu" aria-label={it.label}>
              <Items items={it.items} onClose={onClose} />
            </div>
          )}
        </div>
      ))}
    </>
  );
}

export function ContextMenu({
  x,
  y,
  items,
  label,
  onClose,
}: {
  x: number;
  y: number;
  items: MenuItem[];
  label: string;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const away = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", away);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", away);
      window.removeEventListener("keydown", key);
    };
  }, [onClose]);
  // Keep it on screen.
  const left = Math.min(x, window.innerWidth - 240);
  const top = Math.min(y, window.innerHeight - 40 - items.length * 28);
  return (
    <div
      ref={ref}
      className="ctx-menu"
      role="menu"
      aria-label={label}
      style={{ left, top: Math.max(4, top) }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <Items items={items} onClose={onClose} />
    </div>
  );
}
