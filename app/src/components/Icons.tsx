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
  // Groups (ADR-087): dashed boxes around shapes.
  group: (
    <I>
      <path d="M3 3h18v18H3z" strokeDasharray="3 2" />
      <path d="M7 7h6v5H7zM11 14h6v4h-6z" fill="currentColor" fillOpacity="0.2" />
    </I>
  ),
  detailGroup: (
    <I>
      <path d="M3 3h18v18H3z" strokeDasharray="3 2" />
      <path d="M6 16l4-8 3 5 2-3 3 6M6 18h12" />
    </I>
  ),
  ungroup: (
    <I>
      <path d="M3 3h8v8H3zM13 13h8v8h-8z" strokeDasharray="3 2" />
      <path d="M5 5h4v4H5zM15 15h4v4h-4z" fill="currentColor" fillOpacity="0.2" />
    </I>
  ),
  edit: (
    <I>
      <path d="M4 20l1-5L16 4l4 4L9 19z" />
      <path d="M14 6l4 4" />
    </I>
  ),
  add: (
    <I>
      <path d="M4 4h16v16H4zM12 8v8M8 12h8" />
    </I>
  ),
  // QA/QC (ADR-088).
  qaReview: (
    <I>
      <path d="M8 3h8v3H8zM6 5H4v16h16V5h-2" />
      <path d="M8 13l3 3 5-6" />
    </I>
  ),
  qaCode: (
    <I>
      <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" />
      <path d="M9 12l2 2 4-4" />
    </I>
  ),
  qaWater: (
    <I>
      <path d="M12 3c3 4 6 7.5 6 11a6 6 0 0 1-12 0c0-3.5 3-7 6-11z" />
      <path d="M9 14a3 3 0 0 0 3 3" />
    </I>
  ),
  qaCoord: (
    <I>
      <path d="M4 4h7v7H4zM13 13h7v7h-7z" />
      <path d="M11 7.5h4.5V13M13 16.5H8.5V11" />
    </I>
  ),
  qaReport: (
    <I>
      <path d="M6 3h9l4 4v14H6zM14 3v5h5M9 12h7M9 16h7" />
    </I>
  ),
  filter: (
    <I>
      <path d="M3 4h18l-7 8.5V19l-4 2v-8.5z" fill="currentColor" fillOpacity="0.2" />
    </I>
  ),
  tree: (
    <I>
      <path d="M12 21v-7M12 16l-3-2.5M12 14.5l3-2" />
      <path
        d="M12 3c-2.6 0-4.3 1.8-4.5 3.8C5.8 7.3 4.5 8.8 4.5 10.6c0 2.3 1.9 4 4.3 4h6.4c2.4 0 4.3-1.7 4.3-4 0-1.8-1.3-3.3-3-3.8C16.3 4.8 14.6 3 12 3z"
        fill="currentColor"
        fillOpacity="0.25"
      />
    </I>
  ),
  conifer: (
    <I>
      <path d="M12 21v-3" />
      <path
        d="M12 3l-4 5h2.5L7 12.5h3L6 18h12l-4-5.5h3L13.5 8H16z"
        fill="currentColor"
        fillOpacity="0.25"
      />
    </I>
  ),
  palm: (
    <I>
      <path d="M12.5 21c.5-4 .3-7-.5-10" />
      <path d="M12 11c-2-3-5-3.5-8-2.5M12 11c2.5-2.5 5.5-2.5 8-1M12 11c-1-3-.5-5.5 1-7.5M12 11c-3.5-.5-6 1-7 3.5M12 11c3 0 5.5 1.5 6.5 4" />
    </I>
  ),
  shrub: (
    <I>
      <path
        d="M4 19c-1-3 1-5.5 3.5-5.5.3-2.3 2.2-3.8 4.5-3.8s4.2 1.5 4.5 3.8c2.5 0 4.5 2.5 3.5 5.5z"
        fill="currentColor"
        fillOpacity="0.25"
      />
      <path d="M3 19h18" />
    </I>
  ),
  grass: (
    <I>
      <path d="M3 20h18M6 20c0-4 1-7 3-10M9 20c0-3-.5-6-2.5-8.5M12 20c0-5 .5-9 2-12M15 20c0-3 1.5-6 4-7.5M18 20c0-2-1-4.5-2.5-6" />
    </I>
  ),
  assets: (
    <I>
      <rect x="3" y="3" width="8" height="8" rx="1" />
      <rect x="13" y="3" width="8" height="8" rx="1" />
      <rect x="3" y="13" width="8" height="8" rx="1" />
      <path d="M17 13v8M13 17h8" />
    </I>
  ),
  ground: (
    <I>
      <path d="M2 17l6-4 5 3 4-2.5 5 3.5v3H2z" fill="currentColor" fillOpacity="0.25" />
      <path d="M5 11V9M8 10V7.5M11 11V8.5" />
    </I>
  ),
  region: (
    <I>
      <path d="M4 7l7-3 9 4-2 11-12 1z" strokeDasharray="3 2" />
      <path d="M8 16c1-2 2-3 3-3.5M12 16c.5-2 1.5-3.5 3-4" />
    </I>
  ),
  season: (
    <I>
      <path d="M12 3a9 9 0 100 18 9 9 0 000-18z" />
      <path d="M12 3v18M3 12h18" strokeDasharray="2 2" />
    </I>
  ),
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
  dimLinear: (
    <I>
      <path d="M4 18V9M20 18V5M4 7h16" />
      <path d="M2.5 8.5l3-3M18.5 8.5l3-3" strokeWidth="2" />
    </I>
  ),
  dimAngular: (
    <I>
      <path d="M4 20L20 20M4 20L16 6" />
      <path d="M12 20a8 8 0 0 0-2.2-5.5" />
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
  spotElevation: (
    <I>
      <path d="M3 18h9" />
      <path d="M7.5 18l-3-5h6z" fill="currentColor" stroke="none" />
      <path d="M7.5 13L13 7h8" />
      <path d="M14 4.5h6" strokeWidth="1.2" />
    </I>
  ),
  detailLine: (
    <I>
      <path d="M4 18L20 6" />
      <path d="M4 12h6" strokeDasharray="2 2" />
      <path d="M14 18h6" strokeWidth="2.4" />
    </I>
  ),
  modelLine: (
    <I>
      <path d="M3 16l9-5 9 5-9 5z" strokeWidth="1" strokeDasharray="2 1.5" />
      <path d="M6 16l12-6" strokeWidth="2" />
    </I>
  ),
  spotSlope: (
    <I>
      <path d="M3 17L21 8" />
      <path d="M6 12.5l9-4.5" strokeWidth="1.2" />
      <path
        d="M15 8l-2.8 0.2 1.3 2.4z"
        fill="currentColor"
        stroke="currentColor"
        strokeWidth="0.8"
      />
      <path d="M3 21h18" strokeWidth="1" strokeDasharray="2 2" />
    </I>
  ),
  northArrow: (
    <I>
      <circle cx="12" cy="13" r="7" />
      <path d="M12 6.5l3.2 9.5L12 14z" fill="currentColor" stroke="none" />
      <path d="M12 6.5L8.8 16 12 14" />
      <path d="M10.3 4.2V1.2l3.4 3V1.2" strokeWidth="1.2" />
    </I>
  ),
  graphicScale: (
    <I>
      <path d="M2 11h20v4H2z" />
      <path d="M2 11h5v4H2zM12 11h10v4H12z" fill="currentColor" />
      <path d="M2 8V7M7 8V7M12 8V7M22 8V7" />
    </I>
  ),
  keyPlan: (
    <I>
      <path d="M3 4h18v16H3z" strokeWidth="1.2" />
      <path d="M6 8h9v4h-4v5H6z" />
      <path d="M6 8h5v9H6z" fill="currentColor" fillOpacity="0.35" stroke="none" />
      <path d="M18 7.5l1.4 4H16.6z" fill="currentColor" stroke="none" />
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
  wallOpening: (
    <I>
      <path d="M3 5h18v14H3z" fill="currentColor" fillOpacity="0.85" />
      <path d="M9 17v-5a3 3 0 0 1 6 0v5z" fill="#fff" stroke="#fff" />
    </I>
  ),
  fascia: (
    <I>
      <path d="M3 9h15l3-3" />
      <rect x="15" y="9" width="4" height="7" fill="currentColor" fillOpacity="0.85" />
      <path d="M3 12h12M5 16v4M11 16v4" strokeWidth="1.3" />
    </I>
  ),
  sofa: (
    <I>
      <path d="M4 11V8a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v3" />
      <path
        d="M2 12a2 2 0 0 1 4 0v2h12v-2a2 2 0 0 1 4 0v5H2z"
        fill="currentColor"
        fillOpacity="0.85"
      />
      <path d="M4 17v2M20 17v2" />
    </I>
  ),
  appliance: (
    <I>
      <rect x="5" y="3" width="14" height="18" rx="1.5" />
      <path d="M5 9h14" />
      <path d="M8 5.5v1.5M8 11.5v4" strokeWidth="1.6" />
      <circle cx="15.5" cy="6" r="0.9" fill="currentColor" />
    </I>
  ),
  light: (
    <I>
      <path d="M12 2v5" />
      <path d="M6 13a6 6 0 0 1 12 0z" fill="currentColor" fillOpacity="0.85" />
      <path d="M9 16l-1.5 3M12 16.5v3.5M15 16l1.5 3" strokeWidth="1.3" />
    </I>
  ),
  sun: (
    <I>
      <circle cx="12" cy="12" r="4" fill="currentColor" fillOpacity="0.85" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
    </I>
  ),
  bulb: (
    <I>
      <path d="M9 18h6M10 21h4" />
      <path d="M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.3 1 2.1h5c0-.8.4-1.6 1-2.1A6 6 0 0 0 12 3z" />
    </I>
  ),
  draftingView: (
    <I>
      <rect x="3" y="4" width="18" height="16" rx="1" />
      <path d="M6 16h6l3-5h3M6 12h4" />
      <path d="M13 16l2 2" strokeWidth="1" />
    </I>
  ),
  detailLibrary: (
    <I>
      <rect x="3" y="3" width="8" height="8" />
      <rect x="13" y="3" width="8" height="8" />
      <rect x="3" y="13" width="8" height="8" />
      <rect x="13" y="13" width="8" height="8" />
      <path d="M4 9l3-3 3 3M14 9h6M17 5v4M4 19h6M13.5 19l3-4 3 4" strokeWidth="1" />
    </I>
  ),
  mepMechanical: (
    <I>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 4c2 3 2 5 0 8s-2 5 0 8M4 12c3-2 5-2 8 0s5 2 8 0" strokeWidth="1.1" />
    </I>
  ),
  mepElectrical: (
    <I>
      <path d="M13 3L6 13h5l-1 8 7-10h-5z" />
    </I>
  ),
  mepPlumbing: (
    <I>
      <path d="M5 4h6v5h6v11" />
      <path d="M8 4v5M14 9v11" strokeWidth="1.1" />
      <path d="M17 14c1.5 2 1.5 3 0 4" strokeWidth="1.1" />
    </I>
  ),
  mepTechnology: (
    <I>
      <rect x="4" y="13" width="16" height="7" />
      <path d="M8 16.5h.01M12 16.5h.01" strokeWidth="2" />
      <path d="M7 9a7 7 0 0 1 10 0M9.5 11.5a3.5 3.5 0 0 1 5 0" />
    </I>
  ),
  keynoteElement: (
    <I>
      <rect x="11" y="3" width="10" height="7" />
      <path d="M11 7L4 17" />
      <path d="M3 15h6v6H3z" strokeWidth="1.2" />
    </I>
  ),
  keynoteMaterial: (
    <I>
      <rect x="11" y="3" width="10" height="7" />
      <path d="M11 7L5 16" />
      <path d="M2 14l6 6M2 18l3 3M5 14l4 4" strokeWidth="1.1" />
    </I>
  ),
  keynoteUser: (
    <I>
      <rect x="11" y="3" width="10" height="7" />
      <path d="M11 7L4 18" />
      <circle cx="4" cy="19" r="1.5" />
    </I>
  ),
  keynoteManager: (
    <I>
      <path d="M4 4h16v16H4z" />
      <path d="M7 8h4M9 12h6M9 16h6M7 8v8" strokeWidth="1.2" />
    </I>
  ),
  keynoteLegend: (
    <I>
      <rect x="4" y="3" width="16" height="18" />
      <path d="M4 8h16M9 3v18M11 12h7M11 16h7" strokeWidth="1.1" />
    </I>
  ),
  worksets: (
    <I>
      <rect x="3" y="4" width="11" height="8" />
      <rect x="7" y="8" width="11" height="8" />
      <rect x="10" y="12" width="11" height="8" />
    </I>
  ),
  grayInactive: (
    <I>
      <rect x="3" y="5" width="8" height="14" />
      <rect x="13" y="5" width="8" height="14" strokeDasharray="2 2" />
    </I>
  ),
  structureSuggest: (
    <I>
      <path d="M4 20V8M12 20V8M20 20V8M3 8h18M3 14h18" />
      <circle cx="18" cy="5" r="2.5" />
    </I>
  ),
  structureOverlay: (
    <I>
      <rect x="3" y="3" width="18" height="18" strokeDasharray="2 2" />
      <path d="M3 12h18M12 3v18" />
      <rect x="10" y="10" width="4" height="4" />
    </I>
  ),
  newView: (
    <I>
      <rect x="3" y="5" width="14" height="14" />
      <path d="M6 15l3-4 3 3 2-2 3 3" strokeWidth="1.2" />
      <path d="M19 3v6M16 6h6" />
    </I>
  ),
  saveDetail: (
    <I>
      <path d="M5 3h11l3 3v15H5z" />
      <path d="M8 3v5h7V3M8 21v-7h8v7" />
    </I>
  ),
  detailComponent: (
    <I>
      <rect x="4" y="6" width="7" height="12" />
      <path d="M4 6l7 12M11 6l-7 12" strokeWidth="1" />
      <path d="M14 12h7M14 12l2-3M21 12l-2 3" />
    </I>
  ),
  repeatingDetail: (
    <I>
      <rect x="3" y="8" width="5" height="8" />
      <rect x="9.5" y="8" width="5" height="8" />
      <rect x="16" y="8" width="5" height="8" />
    </I>
  ),
  insulation: (
    <I>
      <path d="M3 17c2-10 4-10 4 0s2-10 4 0 2-10 4 0 2-10 4 0 2-10 2-4" strokeWidth="1.3" />
    </I>
  ),
  filledRegion: (
    <I>
      <path d="M4 18l3-12 13 3-4 11z" />
      <path d="M6 13l5-6M5.5 17l9-10.5M9 19l8-9.5M13 19.5l5-6" strokeWidth="0.9" />
    </I>
  ),
  inPlace: (
    <I>
      <path d="M4 8l7-4 7 4v8l-7 4-7-4z" />
      <path d="M4 8l7 4 7-4M11 12v8" />
      <path d="M15 21l6-6 2 2-6 6h-2z" strokeWidth="1.2" />
    </I>
  ),
  extrusion: (
    <I>
      <path d="M5 16l5 3 9-4-5-3z" />
      <path d="M5 16V8l5 3v8M10 11l9-4v8M5 8l9-4 5 3" />
    </I>
  ),
  blend: (
    <I>
      <path d="M4 18l8 3 8-3-8-3z" />
      <path d="M4 18l5-11h6l5 11M9 7l3 1 3-1" />
    </I>
  ),
  sweep: (
    <I>
      <path d="M3 18c4 0 6-2 8-6s4-6 10-6" strokeDasharray="2 2" />
      <path d="M3 15h3v6H3zM18 3h3v6h-3z" />
      <path d="M6 15c3-1 5-3 6-6s3-4 6-6M6 21c4 0 7-2 8-6s3-6 7-6" />
    </I>
  ),
  voidExtrusion: (
    <I>
      <path d="M5 16l5 3 9-4-5-3zM5 16V8l5 3v8M10 11l9-4v8M5 8l9-4 5 3" strokeDasharray="2 1.6" />
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
  sparkle: (
    <I>
      <path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z" />
      <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z" />
    </I>
  ),
  plans: (
    <I>
      <path d="M3 17l9 4 9-4" />
      <path d="M3 12l9 4 9-4" />
      <path d="M3 7l9-4 9 4-9 4z" />
    </I>
  ),
  paint: (
    <I>
      <rect x="4" y="3" width="14" height="6" rx="1" />
      <path d="M18 6h2v5h-8v3" />
      <rect x="10.5" y="14" width="3" height="7" rx="1" />
    </I>
  ),
  camera: (
    <I>
      <rect x="3" y="8" width="12" height="9" rx="1" />
      <path d="M15 11l6-3v9l-6-3z" />
    </I>
  ),
  render: (
    <I>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4" />
    </I>
  ),
  pin: (
    <I>
      <path d="M9 3h6l-1 6 3 3H7l3-3z" />
      <path d="M12 12v9" />
    </I>
  ),
  unpin: (
    <I>
      <path d="M9 3h6l-1 6 3 3H7l3-3z" />
      <path d="M12 12v9M4 4l16 16" />
    </I>
  ),
  eye: (
    <I>
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
    </I>
  ),
  thin: (
    <I>
      <path d="M4 7h16M4 12h16M4 17h16" strokeWidth="0.8" />
    </I>
  ),
  mapPin: (
    <I>
      <path d="M12 21s-6.5-6.2-6.5-11.2a6.5 6.5 0 0113 0C18.5 14.8 12 21 12 21z" />
      <circle cx="12" cy="9.8" r="2.4" />
    </I>
  ),
  sitePlan: (
    <I>
      <path d="M4 5l7-1 9 3-2 12-12 1z" strokeDasharray="4 1.5 1 1.5" />
      <path d="M9 10h5v5H9z" fill="currentColor" fillOpacity="0.3" />
    </I>
  ),
  topo: (
    <I>
      <path d="M3 17c3-4 6-5 9-3s6 1 9-3" />
      <path d="M3 12c3-4 6-5 9-3s6 1 9-3" strokeWidth="1.1" />
      <path d="M3 21c3-3 6-4 9-2s6 1 9-2" strokeWidth="1.1" />
    </I>
  ),
  key: (
    <I>
      <circle cx="8" cy="15" r="4" />
      <path d="M11 12l9-9M16 7l3 3M14 9l2 2" />
    </I>
  ),
  elevation: (
    <I>
      <circle cx="12" cy="12" r="4.5" />
      <path d="M12 3l3 5H9zM21 12l-5 3V9zM12 21l-3-5h6zM3 12l5-3v6z" fill="currentColor" />
    </I>
  ),
  separator: (
    <I>
      <path d="M3 4h18v16H3z" strokeWidth="1.6" />
      <path d="M12 4v16" strokeWidth="1" strokeDasharray="2 1.5" />
    </I>
  ),
  callout: (
    <I>
      <rect x="3" y="7" width="12" height="12" rx="3" />
      <path d="M15 7l3-3" />
      <circle cx="19.5" cy="3.8" r="2.3" />
    </I>
  ),
  material: (
    <I>
      <path d="M4 4h16v16H4z" />
      <path d="M4 12l8-8M4 20L20 4M12 20l8-8" strokeWidth="1.1" />
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
