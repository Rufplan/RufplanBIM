// Simple line icons (24 × 24, currentColor) for the ribbon.
import type { ReactNode } from "react";

const I = ({ children }: { children: ReactNode }) => (
  <svg
    viewBox="0 0 24 24"
    width="22"
    height="22"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    aria-hidden
  >
    {children}
  </svg>
);

export const Icons = {
  select: (
    <I>
      <path d="M5 3l12 8-5.5 1.2L14 19l-2.4 1.2-2.6-6.6L5 17z" fill="currentColor" stroke="none" />
    </I>
  ),
  wall: (
    <I>
      <path d="M3 8h18v8H3z" fill="currentColor" fillOpacity="0.85" />
      <path d="M3 8l4 8M9 8l4 8M15 8l4 8" stroke="#fff" strokeWidth="1" />
    </I>
  ),
  dimension: (
    <I>
      <path d="M3 8v8M21 8v8M3 12h18" />
      <path d="M1.5 13.5l3-3M19.5 13.5l3-3" strokeWidth="2" />
    </I>
  ),
  text: (
    <I>
      <path d="M5 5h14M12 5v14M9 19h6" strokeWidth="2" />
    </I>
  ),
  section: (
    <I>
      <path d="M4 12h16" strokeDasharray="4 2 1 2" />
      <circle cx="4" cy="12" r="2.6" />
      <path d="M4 6.5l2.5 3h-5z" fill="currentColor" stroke="none" />
    </I>
  ),
  tag: (
    <I>
      <path d="M4 8h11l5 4-5 4H4z" />
      <path d="M8 12h4" strokeWidth="2" />
    </I>
  ),
  issue: (
    <I>
      <path d="M5 4h10l4 4v12H5z" />
      <path d="M8 13l3 3 5-6" stroke="#3ECFF7" strokeWidth="2.2" />
    </I>
  ),
  sheet: (
    <I>
      <path d="M3 5h18v14H3z" />
      <path d="M16 5v14M16 15h5" />
      <path d="M16 7h5" stroke="#3ECFF7" strokeWidth="2.4" />
    </I>
  ),
  place: (
    <I>
      <path d="M3 5h18v14H3z" />
      <path d="M7 9h6v6H7z" fill="currentColor" fillOpacity="0.25" />
    </I>
  ),
  pdf: (
    <I>
      <path d="M6 3h9l4 4v14H6z" />
      <path d="M15 3v4h4" />
      <path d="M9 13h6M9 16h6M9 10h3" strokeWidth="1.2" />
    </I>
  ),
  ifc: (
    <I>
      <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
      <path d="M8 10v5M11 10v5M11 10h3M11 12.5h2.5M18 10h-2.5v5H18" strokeWidth="1.2" />
    </I>
  ),
  account: (
    <I>
      <circle cx="12" cy="8.5" r="3.8" />
      <path d="M4.5 20c1.2-4 4.2-5.6 7.5-5.6s6.3 1.6 7.5 5.6" />
    </I>
  ),
  link: (
    <I>
      <path d="M10 14l4-4" />
      <path d="M8.5 11.5l-2 2a3.2 3.2 0 004.5 4.5l2-2M15.5 12.5l2-2A3.2 3.2 0 0013 6l-2 2" />
    </I>
  ),
  publish: (
    <I>
      <path d="M12 16V4M7 9l5-5 5 5" />
      <path d="M4 14v6h16v-6" />
      <path d="M8 20h8" stroke="#3ECFF7" strokeWidth="2.4" />
    </I>
  ),
  copy: (
    <I>
      <path d="M8 8h11v11H8z" />
      <path d="M5 16V5h11" />
    </I>
  ),
  rotate: (
    <I>
      <path d="M19 12a7 7 0 11-2.05-4.95" />
      <path d="M19 4v4h-4" />
      <circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none" />
    </I>
  ),
  mirror: (
    <I>
      <path d="M12 3v18" strokeDasharray="3 2" />
      <path d="M9 7L4 12l5 5z" fill="currentColor" fillOpacity="0.25" />
      <path d="M15 7l5 5-5 5z" />
    </I>
  ),
  array: (
    <I>
      <path d="M3 9h4v6H3zM10 9h4v6h-4zM17 9h4v6h-4z" />
    </I>
  ),
  align: (
    <I>
      <path d="M4 3v18" strokeWidth="2.2" />
      <path d="M8 7h10M8 12h6M8 17h12" />
      <path d="M11 10l-3 2 3 2" />
    </I>
  ),
  trim: (
    <I>
      <path d="M4 18h10V6" strokeWidth="2.4" />
      <path d="M14 18h6M14 6V2" strokeDasharray="2 2" />
    </I>
  ),
  offset: (
    <I>
      <path d="M4 16l12-12" strokeWidth="2.2" />
      <path d="M8 20l12-12" strokeDasharray="3 2" />
    </I>
  ),
  split: (
    <I>
      <path d="M3 12h7M14 12h7" strokeWidth="2.4" />
      <path d="M13 6l-2 12" />
    </I>
  ),
  flip: (
    <I>
      <path d="M4 9h13l-3-3M20 15H7l3 3" />
    </I>
  ),
  roof: (
    <I>
      <path d="M2 13L12 5l10 8" strokeWidth="2" />
      <path d="M5 11v8h14v-8" />
    </I>
  ),
  column: (
    <I>
      <path d="M8 3h8M8 21h8M10 3v18M14 3v18" strokeWidth="1.8" />
    </I>
  ),
  columnGrid: (
    <I>
      <path d="M3 6h18M3 18h18M6 3v18M18 3v18" strokeWidth="1" strokeDasharray="2 2" />
      <path
        d="M4.5 4.5h3v3h-3zM16.5 4.5h3v3h-3zM4.5 16.5h3v3h-3zM16.5 16.5h3v3h-3z"
        fill="currentColor"
      />
    </I>
  ),
  beam: (
    <I>
      <path d="M3 8h18M3 16h18M12 8v8" strokeWidth="1.8" />
      <path d="M3 6v4M21 6v4M3 14v4M21 14v4" />
    </I>
  ),
  railing: (
    <I>
      <path d="M3 7h18M3 19h18" strokeWidth="1.8" />
      <path d="M5 7v12M9 7v12M13 7v12M17 7v12" strokeWidth="1.1" />
    </I>
  ),
  attach: (
    <I>
      <path d="M3 10L12 4l9 6" strokeWidth="1.8" />
      <path d="M7 20v-8M17 20v-8M12 20v-9" />
      <path d="M10 14l2-2 2 2" />
    </I>
  ),
  stair: (
    <I>
      <path d="M3 20h4v-4h4v-4h4V8h4V4" strokeWidth="1.8" />
    </I>
  ),
  params: (
    <I>
      <path d="M4 5h16v14H4z" />
      <path d="M7 9h4M7 13h6M7 16h3M14 9h3" strokeWidth="1.2" />
    </I>
  ),
  room: (
    <I>
      <path d="M3 4h18v16H3z" strokeWidth="2.2" />
      <path d="M7 11h10M9 15h6" strokeWidth="1.2" />
      <path d="M8 7.5h8" strokeWidth="1.8" />
    </I>
  ),
  move: (
    <I>
      <path d="M12 3v18M3 12h18" />
      <path d="M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3" />
    </I>
  ),
  door: (
    <I>
      <path d="M3 20h5M16 20h5" strokeWidth="2.4" />
      <path d="M8 20V8" strokeWidth="1.8" />
      <path d="M8 8a12 12 0 0112 12" strokeWidth="1" />
    </I>
  ),
  window: (
    <I>
      <path d="M2 10h4M18 10h4M2 14h4M18 14h4" strokeWidth="2.4" />
      <path d="M6 10h12M6 14h12" strokeWidth="1" />
      <path d="M6 11.3h12M6 12.7h12" strokeWidth="0.8" />
    </I>
  ),
  floor: (
    <I>
      <path d="M2 15l6-6h14l-6 6z" />
      <path d="M2 15v3h14l6-6V9" />
    </I>
  ),
  floorAuto: (
    <I>
      <path d="M4 5h16v14H4z" strokeWidth="3" />
      <path d="M8 9h8v6H8z" fill="currentColor" fillOpacity="0.25" stroke="none" />
    </I>
  ),
  ceiling: (
    <I>
      <path d="M3 5h18v14H3z" />
      <path d="M3 10h18M3 15h18M9 5v14M15 5v14" strokeWidth="1" />
    </I>
  ),
  level: (
    <I>
      <path d="M2 14h14" strokeDasharray="4 2 1 2" />
      <circle cx="19" cy="14" r="2.5" fill="currentColor" />
      <path d="M13 9h8" strokeWidth="1.2" />
    </I>
  ),
  grid: (
    <I>
      <path d="M12 8v14" strokeDasharray="4 2 1 2" />
      <circle cx="12" cy="5" r="3.5" />
    </I>
  ),
  del: (
    <I>
      <path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13" />
    </I>
  ),
  view3d: (
    <I>
      <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
      <path d="M12 12l8-4.5M12 12v9M12 12L4 7.5" />
    </I>
  ),
  undo: (
    <I>
      <path d="M9 7L4 12l5 5" />
      <path d="M4 12h10a6 6 0 010 12" transform="translate(0 -6)" />
    </I>
  ),
  redo: (
    <I>
      <path d="M15 7l5 5-5 5" />
      <path d="M20 12H10a6 6 0 000 12" transform="translate(0 -6)" />
    </I>
  ),
  fit: (
    <I>
      <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
    </I>
  ),
};
