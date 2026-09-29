import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppState } from "../ipc";
import type { DetailLevel } from "../bindings/DetailLevel";
import type { FormKind } from "../bindings/FormKind";
import type { FillPattern } from "../bindings/FillPattern";
import type { Category } from "../bindings/Category";
import type { Standards } from "../bindings/Standards";

/** A small Asset Library: a maple in two seasons and a boxwood. */
const plantSpec = (group: string, form: string, season: string) => ({
  botanical: "Acer rubrum",
  group,
  form,
  foliage: "Palmate",
  bark_kind: "Smooth",
  height: 15240,
  spread: 10668,
  depth: 0,
  trunk: 4572,
  caliper: 363,
  stems: 1,
  leaf: [72, 104, 48],
  leaf_alt: [90, 120, 60],
  bark: [118, 114, 106],
  autumn: [178, 48, 30],
  season,
  density: 0.8,
});
export const FAKE_PLANT_LIBRARY = {
  presets: [
    {
      name: "Red Maple",
      species: "Red Maple",
      description: "Fast, adaptable street and yard tree.",
      climates: ["Temperate"],
      spec: plantSpec("Deciduous", "Oval", "Summer"),
    },
    {
      name: "Red Maple (Autumn)",
      species: "Red Maple",
      description: "Fast, adaptable street and yard tree.",
      climates: ["Temperate"],
      spec: plantSpec("Deciduous", "Oval", "Autumn"),
    },
    {
      name: "Boxwood, Round",
      species: "Boxwood, Round",
      description: "Clipped evergreen globe.",
      climates: ["Temperate"],
      spec: { ...plantSpec("Shrub", "Mound", "Summer"), botanical: "Buxus sempervirens" },
    },
  ],
  groups: [
    ["Deciduous", "Deciduous Trees", "Trees"],
    ["Shrub", "Shrubs & Bushes", "Bushes"],
  ],
  climates: [["Temperate", "Temperate"]],
};

export interface FakeBackend {
  calls: { cmd: string; args: unknown }[];
  /** Path returned by the next open dialog; null simulates Cancel. */
  openPath: string | null;
  /** Path returned by the next save dialog; null simulates Cancel. */
  savePath: string | null;
  failWith: string | null;
  state: AppState | null;
  googleKey: string | null;
  regrid: boolean;
  /** Ids last pinned through set_pinned. */
  pinned: string[];
  /** apply_material calls as (ids, material). */
  applied: [string[], string][];
  /** Whether a Claude key is saved, and the last generate inputs. */
  claudeKey: boolean;
  generated: unknown;
  /** The last Plans to 3D inputs. */
  plans: unknown;
  /** Categories to report for selected ids (anything else is a view). */
  properties?: Record<string, { category: string }>;
  /** The fixtures' lights (ADR-057). */
  lights: import("../bindings/LightInfo").LightInfo[];
  /** What pick_cycle finds under the cursor (ADR-056). */
  underCursor: { ids: string[]; label: string }[];
  /** The keynote table and assignments (ADR-081). */
  keynotes: { key: string; text: string; parent?: string }[];
  keynoteNumbering: "ByKeynote" | "BySheet";
  keynoteAssigned: Record<string, string>;
  /** The project's drawing-set standards (ADR-047). */
  standards: Standards;
}

/** Two categories of the default standards, one item open in each. */
export function fakeStandards(library = "Rufplan Default (NCS 6)"): Standards {
  return {
    library,
    categories: [
      {
        id: "sheet",
        label: "Sheet Setup",
        short: "Sheet Setup",
        group: "SHEETS",
        applies: ["Sheets", "Title Blocks"],
        items: [
          {
            name: "Sheet size",
            value: library.startsWith("Res") ? "ARCH C" : "ARCH D",
            options: ["ARCH C", "ARCH D"],
            done: true,
          },
          { name: "Revision block", value: "", options: [], done: false },
        ],
      },
      {
        id: "dim",
        label: "Dimensions",
        short: "Dims",
        group: "ANNOTATION",
        applies: ["Dimensions"],
        items: [{ name: "Wall dimension to", value: "", options: [], done: false }],
      },
    ],
  };
}

const ids = {
  plan1: "00000000-0000-7000-8000-000000000001",
  rcp1: "00000000-0000-7000-8000-000000000002",
  north: "00000000-0000-7000-8000-000000000003",
  v3d: "00000000-0000-7000-8000-000000000004",
  l1: "00000000-0000-7000-8000-000000000010",
  wt: "00000000-0000-7000-8000-000000000020",
  ft: "00000000-0000-7000-8000-000000000021",
  ct: "00000000-0000-7000-8000-000000000022",
  dt: "00000000-0000-7000-8000-000000000023",
  wnt: "00000000-0000-7000-8000-000000000024",
  sd: "00000000-0000-7000-8000-000000000030",
  dd: "00000000-0000-7000-8000-000000000031",
  info: "00000000-0000-7000-8000-000000000040",
};
export const FAKE_IDS = ids;

export function appState(path: string | null, dirty = false): AppState {
  const name = path ? (path.split(/[\\/]/).pop() ?? "").replace(/\.rfproj$/, "") : "Untitled";
  const v = (
    id: string,
    n: string,
    viewType: AppState["views"][number]["viewType"],
    level: string | null,
  ) => ({
    id,
    name: n,
    viewType,
    scale: 48,
    scaleLabel: `1/4" = 1'-0"`,
    level,
    onSheet: null,
    stages: [],
    sectionBox: null,
    calloutOf: null,
    hiddenCategories: [],
    hiddenCount: 0,
    hiddenWorksets: [] as string[],
    site: false,
    camera: null as AppState["views"][number]["camera"],
    detailLevel: (viewType === "ThreeD" ? null : "Fine") as DetailLevel | null,
  });
  return {
    project: { name, path, schemaVersion: 2, appVersion: "0.0.1", dirty },
    revision: 1,
    views: [
      v(ids.plan1, "Level 1", "Plan", ids.l1),
      v(ids.rcp1, "Level 1", "CeilingPlan", ids.l1),
      v(ids.north, "North", "Elevation", null),
      v(ids.v3d, "{3D}", "ThreeD", null),
    ],
    levels: [{ id: ids.l1, name: "Level 1" }],
    wallTypes: [{ id: ids.wt, name: `Exterior - 8" Stud` }],
    floorTypes: [{ id: ids.ft, name: `Concrete Slab - 6"` }],
    ceilingTypes: [{ id: ids.ct, name: "ACT 2x4 Ceiling" }],
    doorTypes: [{ id: ids.dt, name: `Single Flush 36" x 84"` }],
    windowTypes: [{ id: ids.wnt, name: `Fixed 48" x 48"` }],
    stages: [
      { id: ids.sd, name: "Schematic Design", abbreviation: "SD" },
      { id: ids.dd, name: "Design Development", abbreviation: "DD" },
    ],
    currentStage: ids.sd,
    projectInfo: ids.info,
    projectName: "New Project",
    undo: null,
    redo: null,
    issuances: [],
    rufplan: null,
    roofTypes: [],
    sketch: null,
    elevationMarkerTypes: [
      { id: "00000000-0000-7000-8000-000000000060", name: "Interior Elevation" },
      { id: "00000000-0000-7000-8000-000000000061", name: "Building Elevation" },
    ],
    levelElevations: [0],
    site: null,
    columnTypes: [{ id: "00000000-0000-7000-8000-000000000025", name: "Steel W10x33" }],
    beamTypes: [{ id: "00000000-0000-7000-8000-000000000026", name: "Steel W12x26" }],
    railingTypes: [{ id: "00000000-0000-7000-8000-000000000027", name: 'Guardrail - 42"' }],
    lightingFixtureTypes: [
      { id: "00000000-0000-7000-8000-000000000041", name: "2x4 LED Troffer" },
      { id: "00000000-0000-7000-8000-000000000042", name: '6" LED Downlight' },
      { id: "00000000-0000-7000-8000-000000000043", name: "Wall Sconce" },
    ],
    sun: { mode: "Still", month: 6, day: 21, hour: 15, azimuth: 225, altitude: 35 },
    plantingTypes: [],
    ground: null,
    materials: [
      { id: "00000000-0000-7000-8000-000000000050", name: "Brick" },
      { id: "00000000-0000-7000-8000-000000000051", name: "Concrete" },
    ],
    paramDefs: [],
    inPlace: null,
    worksets: FAKE_WORKSETS.map(({ id, name }) => ({ id, name })),
    activeWorkset: FAKE_WORKSETS[0]!.id,
    structuralLayer: null,
    mepLayers: [],
  };
}

/** A MEPT Suggest answer (ADR-082). */
export const FAKE_MEP_PROPOSAL = (discipline: string, climate: string) => {
  const sys = (key: string, label: string, score: number) => ({
    key,
    label,
    description: `${label} description.`,
    score,
    ruledOut: false,
    criteria: [{ name: "Size", score: 1, note: "Fits." }],
    rationale: `${label} scores ${score}/100.`,
    highlights: [`${discipline} key number`],
    redFlags: key === "b" ? ["A red flag."] : [],
    settings: { discipline, system: key, climate },
  });
  return {
    discipline,
    disclaimer: `Preliminary — not engineered. Requires review by a licensed ${discipline.toLowerCase()} engineer.`,
    climate,
    systems: [
      sys("a", `${discipline} System A`, 90),
      sys("b", `${discipline} System B`, 80),
      sys("c", `${discipline} System C`, 70),
      sys("d", `${discipline} System D`, 60),
    ],
    assumptions: ["An assumption."],
    questions: ["A question?"],
    summary: ["2 stories."],
  };
};

/** A small keynote table (ADR-081). */
export const FAKE_KEYNOTES = [
  { key: "03", text: "Concrete" },
  { key: "03 30 00", text: "Cast-in-Place Concrete", parent: "03" },
  { key: "03 30 00.A1", text: '4" concrete slab on grade', parent: "03 30 00" },
  { key: "09", text: "Finishes" },
  { key: "09 29 00", text: "Gypsum Board", parent: "09" },
  { key: "09 29 00.A1", text: '1/2" gypsum board', parent: "09 29 00" },
  { key: "09 29 00.A2", text: '5/8" Type X gypsum board', parent: "09 29 00" },
];

/** A Suggest Structure answer (ADR-080). */
export const FAKE_PROPOSAL = (seismic: string) => {
  const scheme = (kind: string, label: string, score: number) => ({
    kind,
    label,
    score,
    ruledOut: false,
    criteria: [{ name: "Height", score: 1, note: "Within its limits." }],
    rationale: `${label} scores ${score}/100.`,
    grid: "Bearing walls about 16' apart",
    memberDepths: ['Joists: 11-7/8" I-joist @ 16" o.c.'],
    redFlags:
      kind === "LightWood" ? ["Transfer beams likely at Level 2 (1 wall doesn't stack)."] : [],
    settings: {
      kind,
      seismic,
      gridX: 4876.8,
      gridY: 4876.8,
      lateral: kind === "SteelFrame" ? "BracedFrames" : "WoodShearWalls",
      spanDir: "Auto",
    },
    laterals: kind === "SteelFrame" ? ["BracedFrames", "MomentFrames"] : ["WoodShearWalls"],
  });
  return {
    disclaimer: "Preliminary — not engineered. Requires review by a licensed structural engineer.",
    seismic,
    schemes: [
      scheme("LightWood", "Light Wood Frame + Wood Shear Walls", 88),
      scheme("ColdFormedSteel", "Cold-Formed Steel Bearing Walls", 80),
      scheme("SteelFrame", "Steel Frame on Composite Deck", 71),
      scheme("MassTimber", "Mass Timber (Glulam + CLT)", 64),
    ],
    assumptions: ["Seismic region: " + seismic.toLowerCase() + " (your setting)."],
    questions: ["Is the site in a low, moderate or high seismic region?"],
    summary: ["3 stories, 30' to the top of the walls."],
  };
};

/** Revit's standard worksets (ADR-079). */
export const FAKE_WORKSETS = [
  ["Architecture", "Default"],
  ["Shared Levels and Grids", "LevelsGrids"],
  ["Structural", "Structural"],
  ["Interiors", "Other"],
  ["Site", "Other"],
  ["MEP", "Other"],
  ["Linked Models", "Other"],
].map(([name, role], i) => ({
  id: `00000000-0000-7000-8000-0000000007${String(i).padStart(2, "0")}`,
  name: name!,
  role: role as "Default" | "LevelsGrids" | "Structural" | "Other",
  visibleInAllViews: true,
  count: i === 0 ? 4 : 0,
}));

/** The fake Detail Library (ADR-069). */
export const FAKE_DETAILS = [
  {
    id: "slab-edge",
    name: "Thickened Slab Edge",
    category: "Foundations",
    scale: 16,
    scaleLabel: `3/4" = 1'-0"`,
    description: "Slab-on-grade with a thickened edge.",
    user: false,
  },
  {
    id: "window-head",
    name: "Window Head - Wood Frame",
    category: "Openings",
    scale: 4,
    scaleLabel: `3" = 1'-0"`,
    description: "Flanged window under an insulated header.",
    user: false,
  },
  {
    id: "eave",
    name: "Eave with Gutter",
    category: "Roofs",
    scale: 8,
    scaleLabel: `1 1/2" = 1'-0"`,
    description: "6:12 truss roof eave.",
    user: false,
  },
];

/** The fake in-place element (ADR-068). */
export const IN_PLACE_ID = "00000000-0000-7000-8000-000000000070";

const appearance = {
  preset: "wood-white-oak-floor",
  roughness: 0.55,
  metalness: 0,
  reflection: 0.5,
  refraction: 0,
  ior: 1.5,
  bump: 1,
  texture: "wood_floor",
  scale: 1700,
  tint: [255, 255, 255] as [number, number, number],
  textureColor: true,
  coat: 0.15,
  sheen: 0,
};
/** A small material library: a high-end wood and a typical metal. */
export const LIBRARY = [
  {
    id: "wood-white-oak-floor",
    name: "White Oak Plank Flooring, Matte",
    category: "Wood",
    tier: "HighEnd" as const,
    residential: true,
    hospitality: true,
    multifamily: true,
    description: "Wide-plank white oak.",
    color: [176, 140, 100] as [number, number, number],
    surface: "none",
    appearance,
  },
  {
    id: "metal-anodized-clear",
    name: "Clear Anodized Aluminum",
    category: "Metal",
    tier: "Typical" as const,
    residential: false,
    hospitality: true,
    multifamily: true,
    description: "Storefront and window frames.",
    color: [190, 192, 196] as [number, number, number],
    surface: "none",
    appearance: {
      ...appearance,
      preset: "metal-anodized-clear",
      texture: null,
      metalness: 1,
      roughness: 0.35,
      reflection: 1,
      coat: 0,
    },
  },
];

const thumb = (w: number, h: number) => ({
  width: w,
  height: h,
  lines: [
    {
      pts: [
        [0, 0],
        [w, 0],
        [w, h],
        [0, h],
      ],
      closed: true,
      dashed: false,
      w: 2,
      glass: false,
    },
    {
      pts: [
        [50, 50],
        [w - 50, 50],
        [w - 50, h - 50],
        [50, h - 50],
      ],
      closed: true,
      dashed: false,
      w: 1,
      glass: true,
    },
  ],
});
const spec = (family: string, units: number, w: number, h: number) => ({
  family,
  units,
  width: w * 25.4,
  height: h * 25.4,
  sill: (84 - h) * 25.4,
  grille: "None",
  finish: "White",
});
const door = (family: string, leaf: string, panels: number, w: number, h: number) => ({
  family,
  leaf,
  panels,
  width: w * 25.4,
  height: h * 25.4,
  finish: null,
});
/** A few families and sizes of the Door Library (ADR-033). */
export const DOOR_LIBRARY = {
  families: [
    {
      family: "SingleFlush",
      label: "Single Swing",
      description: "One hinged leaf.",
      leaves: [
        { id: "Flush", label: "Flush" },
        { id: "SixPanel", label: "Six-Panel" },
      ],
      panels: null,
      panelsLabel: "",
      defaultFinish: "PaintedWhite",
    },
    {
      family: "SlidingGlass",
      label: "Sliding Glass Patio",
      description: "Sliding glass panels.",
      leaves: [],
      panels: [2, 4],
      panelsLabel: "Panels",
      defaultFinish: "PaintedWhite",
    },
  ],
  presets: [
    { spec: door("SingleFlush", "Flush", 0, 36, 84), name: 'Single Flush 36" x 84"' },
    { spec: door("SingleFlush", "SixPanel", 0, 32, 80), name: 'Single Six-Panel 32" x 80"' },
    { spec: door("SlidingGlass", "FullLite", 2, 72, 80), name: 'Sliding Glass 72" x 80"' },
  ],
  finishes: [
    { id: "PaintedWhite", label: "Painted White", color: [238, 238, 234] },
    { id: "Walnut", label: "Walnut", color: [104, 70, 46] },
  ],
};

/** A few families and sizes of the Window Library (ADR-031). */
export const WINDOW_LIBRARY = {
  families: [
    {
      family: "DoubleHung",
      label: "Double-Hung",
      description: "Two sashes that both slide.",
      mullable: true,
      grilles: true,
    },
    {
      family: "Casement",
      label: "Casement",
      description: "A side-hinged sash.",
      mullable: true,
      grilles: true,
    },
    {
      family: "Storefront",
      label: "Storefront",
      description: "Aluminum-framed glazing.",
      mullable: false,
      grilles: false,
    },
  ],
  presets: [
    {
      spec: spec("DoubleHung", 1, 30, 60),
      name: 'Double Hung 30" x 60"',
      preview: thumb(762, 1524),
    },
    {
      spec: spec("DoubleHung", 1, 36, 60),
      name: 'Double Hung 36" x 60"',
      preview: thumb(914, 1524),
    },
    { spec: spec("Casement", 1, 24, 48), name: 'Casement 24" x 48"', preview: thumb(610, 1219) },
  ],
  grilles: ["None", "Colonial", "Prairie", "Craftsman"].map((id) => ({ id, label: id })),
  finishes: [
    { id: "White", label: "White", color: [240, 240, 236] },
    { id: "Black", label: "Black", color: [34, 34, 34] },
  ],
};

/** Installs an in-memory stand-in for the Rust commands and dialog plugin. */
export function installFakeBackend(): FakeBackend {
  const fake: FakeBackend = {
    calls: [],
    openPath: null,
    savePath: null,
    failWith: null,
    state: null,
    googleKey: null,
    regrid: false,
    pinned: [],
    applied: [],
    claudeKey: false,
    generated: null,
    plans: null,
    standards: fakeStandards(),
    underCursor: [],
    keynotes: FAKE_KEYNOTES.map((k) => ({ ...k })),
    keynoteNumbering: "ByKeynote",
    keynoteAssigned: {},
    lights: [],
  };

  mockIPC(
    (cmd, args) => {
      fake.calls.push({ cmd, args });
      if (fake.failWith && !cmd.startsWith("plugin:")) throw { message: fake.failWith };
      const a = args as Record<string, unknown>;
      const bump = () => {
        if (fake.state) fake.state = { ...fake.state, revision: fake.state.revision + 1 };
        return fake.state;
      };
      switch (cmd) {
        case "app_state":
          return fake.state;
        case "project_new":
          return (fake.state = appState(null));
        case "project_import_ifc":
          fake.state = appState(null);
          return {
            state: fake.state,
            report: {
              schema: "IFC2X3",
              application: "Autodesk Revit Architecture 2011 - 1.0",
              project: "0001",
              levels: 4,
              walls: 57,
              doors: 14,
              windows: 22,
              floors: 20,
              roofs: 1,
              rooms: 21,
              grids: 0,
              columns: 0,
              skipped: [["stair", 2]],
              warnings: [
                "2 doors and windows have no wall to go in (skylights, curtain wall doors) and were left out",
              ],
            },
          };
        case "project_open":
          return (fake.state = appState(a.path as string));
        case "project_save":
          return (fake.state = appState(
            (a.path as string | null) ?? fake.state?.project.path ?? null,
          ));
        case "set_property":
          if (fake.state && a.key === "current_stage")
            fake.state = { ...fake.state, currentStage: a.value as string };
          if (fake.state && a.key === "detail_level")
            fake.state = {
              ...fake.state,
              views: fake.state.views.map((v) =>
                v.id === a.id ? { ...v, detailLevel: a.value as DetailLevel } : v,
              ),
            };
          return fake.state;
        case "view_display_list":
          return { viewType: "Plan", scale: 48, bounds: [0, 0, 10000, 8000], items: [] };
        case "properties":
          return {
            id: a.id,
            category: fake.properties?.[a.id as string]?.category ?? "View",
            title: "Level 1",
            typeId: null,
            properties: [],
          };
        case "handles":
          return { grips: [], dims: [] };
        case "create_camera":
          if (fake.state) {
            const eye = a.eye as { x: number; y: number };
            const target = a.target as { x: number; y: number };
            const h = a.height as number;
            const base = fake.state.views.find((x) => x.viewType === "ThreeD")!;
            fake.state = {
              ...fake.state,
              views: [
                ...fake.state.views,
                {
                  ...base,
                  id: "00000000-0000-7000-8000-0000000000c1",
                  name: "3D View 1",
                  camera: { eye: [eye.x, eye.y, h], target: [target.x, target.y, h], fov: 50 },
                },
              ],
            };
          }
          return fake.state;
        case "sun_position":
          return { dir: [0.3, -0.5, 0.8], altitude: 53, azimuth: 211 };
        case "site_imagery_frame":
          return {
            lat: 37.7773,
            lon: -122.462,
            zoom: 20,
            width: 400,
            height: 400,
            corners: [
              { x: -23000, y: -23000 },
              { x: 23000, y: -23000 },
              { x: 23000, y: 23000 },
              { x: -23000, y: 23000 },
            ],
          };
        // Edit Model (ADR-050): a doors edit, or a refusal for anything "blue".
        case "model_edit_preview": {
          const prompt = String(a.prompt);
          const empty = { steps: [], created: [], changed: [], deleted: [] };
          if (prompt.includes("blue"))
            return {
              edit: { operations: [], summary: "", message: "The model has no blue material." },
              preview: null,
              error: "The model has no blue material.",
            };
          if (prompt.includes("office"))
            return {
              edit: { operations: [{ op: "create_wall" }], summary: "Adds an office", message: "" },
              preview: {
                ...empty,
                category: "",
                parameter: "",
                from: "",
                to: "",
                scope: "",
                count: 6,
                ids: ["window-9"],
                summary: "Adds a 12' x 10' office",
                steps: [
                  "Created 1 Wall",
                  "Created 1 Wall",
                  "Created 1 Door",
                  "Deleted 1 element(s)",
                ],
                created: ["2 Walls", "1 Doors"],
                changed: [],
                deleted: ["1 Windows"],
              },
              error: null,
            };
          return {
            edit: { operations: [{ op: "set_parameter" }], summary: "", message: "" },
            preview: {
              ...empty,
              category: "Doors",
              parameter: "Width",
              from: "2'-8\"",
              to: "3'-0\"",
              scope: "Entire model",
              count: 2,
              ids: ["door-1", "door-2"],
              summary: "Width → 3'-0\" on 2 doors",
              steps: ["Width → 3'-0\" on 2 doors"],
            },
            error: null,
          };
        }
        case "model_edit_apply":
          if (fake.state)
            fake.state = {
              ...fake.state,
              revision: fake.state.revision + 1,
              undo: "Edit model: Width → 3'-0\" on 2 doors",
              redo: null,
            };
          return fake.state;
        case "undo":
          if (fake.state) fake.state = { ...fake.state, redo: fake.state.undo, undo: null };
          return fake.state;
        case "redo":
          if (fake.state) fake.state = { ...fake.state, undo: fake.state.redo, redo: null };
          return fake.state;
        case "standards_get":
          return fake.standards;
        case "standards_choices":
          return a.category === "sheet" && a.index === 0
            ? [
                { label: "ARCH C", detail: "Houses.", preview: null },
                { label: "ARCH D", detail: "The usual set.", preview: null },
              ]
            : [];
        case "standards_libraries":
          return ["Rufplan Default (NCS 6)", "Residential", "Preservation"];
        case "standards_set": {
          const cat = fake.standards.categories.find((c) => c.id === a.category)!;
          const item = cat.items[a.index as number]!;
          if (a.value !== null) {
            item.value = a.value as string;
            if (item.value) item.done = true;
          }
          if (a.done !== null) item.done = a.done as boolean;
          fake.standards = structuredClone(fake.standards);
          if (fake.state) fake.state = { ...fake.state, revision: fake.state.revision + 1 };
          return fake.state;
        }
        case "standards_load_library":
          fake.standards = fakeStandards(a.name as string);
          if (fake.state) fake.state = { ...fake.state, revision: fake.state.revision + 1 };
          return fake.state;
        case "site_keys":
          return { googleKey: fake.googleKey, regrid: fake.regrid };
        case "site_set_keys":
          if (a.google !== null) fake.googleKey = (a.google as string) || null;
          if (a.regrid !== null) fake.regrid = !!a.regrid;
          return { googleKey: fake.googleKey, regrid: fake.regrid };
        case "sketch_begin":
          if (fake.state)
            fake.state = {
              ...fake.state,
              sketch: {
                kind: a.kind as "Floor" | "Ceiling" | "WallOpening" | "FilledRegion",
                // From 3D, the sketch goes through the level's plan (ADR-025); a wall
                // opening's stays in its view, on the wall's face (ADR-058).
                view:
                  a.view === ids.v3d && a.kind !== "WallOpening" ? ids.plan1 : (a.view as string),
                wall:
                  a.kind === "WallOpening"
                    ? {
                        frame: {
                          wall: (a.host as string | null) ?? "w1",
                          start: { x: 0, y: 0 },
                          dir: { x: 1, y: 0 },
                          normal: { x: 0, y: -1 },
                          half: 100,
                          base_z: 0,
                          length: 6000,
                          height: 3000,
                        },
                        a: 0,
                        s: 1,
                        zOff: 0,
                      }
                    : null,
                level: (a.level as string | null) ?? ids.l1,
                elevation: 0,
                target: (a.target as string | null) ?? null,
                typeId: (a.typeId as string | null) ?? ids.ft,
                curves: [],
                bad: [],
                error: null,
                canUndo: false,
                canRedo: false,
                form: null,
                region: a.kind === "FilledRegion" ? "Diagonal" : null,
              },
            };
          return fake.state;
        // The Details tab (ADR-069).
        case "detail_library":
          return FAKE_DETAILS;
        case "detail_preview":
          return {
            viewType: "Drafting",
            scale: 4,
            bounds: [0, 0, 1000, 600],
            items: [
              {
                el: null,
                prim: {
                  t: "Fill",
                  rings: [
                    [
                      [0, 0],
                      [500, 0],
                      [500, 200],
                    ],
                  ],
                  fill: "Paper",
                },
              },
              {
                el: null,
                prim: {
                  t: "Line",
                  pts: [
                    [0, 0],
                    [900, 500],
                  ],
                  closed: false,
                  w: 3,
                  dash: "Solid",
                },
              },
              { el: null, prim: { t: "Circle", c: [300, 300], r: 20, w: 1, filled: true } },
              {
                el: null,
                prim: {
                  t: "Text",
                  at: [600, 100],
                  text: "NOTE",
                  size: 10,
                  anchor: "Left",
                  angle: 0,
                },
              },
            ],
          };
        // MEPT (ADR-082).
        case "mep_suggest":
          return FAKE_MEP_PROPOSAL(a.discipline as string, a.climate as string);
        case "mep_generate": {
          const d = (a.settings as { discipline: string }).discipline;
          if (fake.state)
            fake.state = {
              ...fake.state,
              revision: fake.state.revision + 1,
              mepLayers: [...new Set([...fake.state.mepLayers, d])] as typeof fake.state.mepLayers,
            };
          return fake.state;
        }
        case "mep_overlay_2d":
          return (a.disciplines as string[]).length
            ? [
                {
                  kind: "Mep",
                  mep: "Diffuser",
                  member: 0,
                  fill: [
                    [
                      [0, 0],
                      [300, 0],
                      [300, 300],
                      [0, 300],
                    ],
                  ],
                  lines: [],
                },
              ]
            : [];
        case "mep_overlay_3d":
          return [];
        case "mep_pick":
          return {
            kind: "Mep",
            mep: "Diffuser",
            title: "Supply Diffuser — 150 cfm diffuser (prelim.)",
            lines: ["Rule: 150 cfm per diffuser."],
          };
        case "mep_info":
          return null;
        case "mep_export_json":
          return a.path;
        case "mep_edit_rules":
          return "C:/data/mep_rules.toml";
        // Keynotes (ADR-081).
        case "keynote_table":
          return {
            entries: fake.keynotes,
            numbering: fake.keynoteNumbering,
            usage: Object.values(fake.keynoteAssigned).map((key) => ({
              key,
              tags: 0,
              assigned: ["a type"],
            })),
          };
        case "keynote_save": {
          const e = a.entry as { key: string; text: string; parent?: string };
          const i = fake.keynotes.findIndex((k) => k.key === (a.oldKey ?? e.key));
          if (i >= 0 && a.oldKey) fake.keynotes[i] = e;
          else fake.keynotes.push(e);
          return bump();
        }
        case "keynote_delete":
          fake.keynotes = fake.keynotes.filter((k) => k.key !== a.key && k.parent !== a.key);
          return bump();
        case "keynote_set_numbering":
          fake.keynoteNumbering = a.numbering as "ByKeynote" | "BySheet";
          return bump();
        case "keynote_assign":
          for (const id of a.ids as string[]) {
            if (a.key) fake.keynoteAssigned[id] = a.key as string;
            else delete fake.keynoteAssigned[id];
          }
          return bump();
        case "keynote_assignables":
          return [
            {
              id: ids.wt,
              category: "WallType",
              name: `Exterior - 8" Stud`,
              key: fake.keynoteAssigned[ids.wt],
            },
            {
              id: "mat-gyp",
              category: "Material",
              name: "Gypsum Wall Board",
              key: fake.keynoteAssigned["mat-gyp"],
            },
          ];
        case "keynote_target":
          return a.id === "w1"
            ? {
                id: "w1",
                typeId: ids.wt,
                typeName: `Exterior - 8" Stud`,
                typeKey: fake.keynoteAssigned[ids.wt],
                materials: [
                  {
                    id: "mat-gyp",
                    category: "Material",
                    name: "Gypsum Wall Board",
                    key: fake.keynoteAssigned["mat-gyp"],
                  },
                  { id: "mat-osb", category: "Material", name: "OSB", key: undefined },
                ],
              }
            : null;
        case "keynote_place":
        case "keynote_import":
          return cmd === "keynote_import" ? [3, bump()] : bump();
        case "keynote_export":
          return a.path;
        case "keynote_legend": {
          if (!fake.state) return [null, null];
          const legend = {
            ...fake.state.views[0]!,
            id: "legend-1",
            name: "Keynote Legend",
            viewType: "Schedule" as const,
            level: null,
          };
          fake.state = {
            ...fake.state,
            revision: fake.state.revision + 1,
            views: [...fake.state.views, legend],
          };
          return ["legend-1", fake.state];
        }
        // Suggest Structure (ADR-080).
        case "structural_suggest":
          return FAKE_PROPOSAL(a.seismic as string);
        case "structural_generate":
          if (fake.state) fake.state = { ...fake.state, structuralLayer: "layer-1" };
          return fake.state;
        case "structural_overlay_2d":
          return fake.state?.structuralLayer
            ? [
                {
                  kind: "Column",
                  member: 0,
                  fill: [
                    [
                      [0, 0],
                      [300, 0],
                      [300, 300],
                      [0, 300],
                    ],
                  ],
                  lines: [],
                },
              ]
            : [];
        case "structural_overlay_3d":
          return [];
        case "structural_pick":
          return fake.state?.structuralLayer
            ? {
                kind: "Column",
                title: "Column — W10x49 (prelim.)",
                lines: ["Height: 10'-0\"", "Rule: At a grid intersection."],
              }
            : null;
        case "structural_export_json":
        case "structural_export_ifc":
          return a.path;
        case "structural_edit_rules":
          return "C:/data/structural_rules.toml";
        // Worksets (ADR-079).
        case "worksets_list":
          return FAKE_WORKSETS;
        case "element_worksets":
          return [["w1", FAKE_WORKSETS[0]!.id]];
        case "set_active_workset":
          if (fake.state) fake.state = { ...fake.state, activeWorkset: a.ws as string };
          return fake.state;
        case "create_workset":
        case "rename_workset":
        case "delete_workset":
        case "set_workset_visible_in_all_views":
        case "set_elements_workset":
          return fake.state;
        case "set_workset_visible_in_view":
          if (fake.state)
            fake.state = {
              ...fake.state,
              views: fake.state.views.map((v) =>
                v.id === a.view
                  ? {
                      ...v,
                      hiddenWorksets: a.visible
                        ? v.hiddenWorksets.filter((w) => w !== a.ws)
                        : [...v.hiddenWorksets, a.ws as string],
                    }
                  : v,
              ),
            };
          return fake.state;
        // Reference sections and callouts (ADR-076).
        case "reference_targets":
          return (fake.state?.views ?? [])
            .filter((v) => v.viewType === "Drafting" || (!a.callout && v.viewType === "Section"))
            .map((v) => ({
              id: v.id,
              label: `${v.viewType === "Drafting" ? "Drafting View" : "Section"}: ${v.name}`,
            }));
        case "create_reference":
          return fake.state;
        case "reference_target":
          return a.id === "ref-1"
            ? (fake.state?.views.find((v) => v.viewType === "Drafting")?.id ?? null)
            : null;
        case "create_plan_view":
        case "create_3d_view":
        case "duplicate_view":
        case "duplicate_sheet": {
          // Views and sheets (ADR-074): named as Revit names them.
          if (!fake.state) return [null, null];
          const views = fake.state.views;
          const src =
            cmd === "create_3d_view"
              ? views.find((v) => v.viewType === "ThreeD")!
              : cmd === "create_plan_view"
                ? views.find((v) => v.viewType === (a.ceiling ? "CeilingPlan" : "Plan"))!
                : views.find((v) => v.id === (a.view ?? a.sheet))!;
          const id = `00000000-0000-7000-8000-0000000008${String(views.length).padStart(2, "0")}`;
          const name =
            cmd === "create_3d_view"
              ? "3D View 1"
              : cmd === "create_plan_view"
                ? `${src.name} (1)`
                : `${src.name} Copy 1`;
          fake.state = {
            ...fake.state,
            revision: fake.state.revision + 1,
            views: [...views, { ...src, id, name }],
          };
          return [id, fake.state];
        }
        case "detail_insert":
        case "create_drafting_view": {
          if (!fake.state) return [null, null];
          const info = FAKE_DETAILS.find((d) => d.id === a.id);
          const id = `00000000-0000-7000-8000-0000000009${String(fake.state.views.length).padStart(2, "0")}`;
          const name = cmd === "detail_insert" ? info!.name : (a.name as string) || "Drafting 1";
          fake.state = {
            ...fake.state,
            revision: fake.state.revision + 1,
            views: [
              ...fake.state.views,
              {
                ...fake.state.views[0]!,
                id,
                name,
                viewType: "Drafting",
                level: null,
                scale: cmd === "detail_insert" ? info!.scale : (a.scale as number),
                scaleLabel: cmd === "detail_insert" ? info!.scaleLabel : "",
                detailLevel: null,
              },
            ],
          };
          return [id, fake.state];
        }
        case "detail_save": {
          const d = {
            id: "user:u1",
            name: a.name as string,
            category: a.category as string,
            scale: 4,
            scaleLabel: `3" = 1'-0"`,
            description: a.description as string,
            user: true,
          };
          FAKE_DETAILS.push(d);
          return d;
        }
        case "detail_delete": {
          const i = FAKE_DETAILS.findIndex((d) => d.id === a.id);
          if (i >= 0) FAKE_DETAILS.splice(i, 1);
          return null;
        }
        case "sketch_set_pattern":
          if (fake.state?.sketch)
            fake.state = {
              ...fake.state,
              sketch: { ...fake.state.sketch, region: a.pattern as FillPattern },
            };
          return fake.state;
        // Model In-Place (ADR-068).
        case "in_place_categories":
          return [
            { category: "Casework", label: "Casework" },
            { category: "Door", label: "Doors" },
            { category: "Furniture", label: "Furniture" },
            { category: "GenericModel", label: "Generic Models" },
            { category: "Wall", label: "Walls" },
          ];
        case "in_place_default_name":
          return `${a.category === "GenericModel" ? "Generic Models" : (a.category as string)} 1`;
        case "in_place_of":
          return (a.ids as string[]).filter((i) => i === IN_PLACE_ID);
        case "in_place_begin":
        case "in_place_edit":
          if (fake.state)
            fake.state = {
              ...fake.state,
              inPlace: {
                id: IN_PLACE_ID,
                name: (a.name as string | null) ?? "Generic Models 1",
                category: (a.category as Category | undefined) ?? "Casework",
                categoryLabel:
                  a.category === "Wall" ? "Walls" : ((a.category as string) ?? "Casework"),
                forms: cmd === "in_place_edit" ? ["Extrusion 1"] : [],
                isNew: cmd === "in_place_begin",
              },
            };
          return fake.state;
        case "in_place_form_begin":
          if (fake.state?.inPlace) {
            const kind = a.kind as string;
            fake.state = {
              ...fake.state,
              sketch: {
                kind: "InPlace",
                view: a.view === ids.v3d ? ids.plan1 : (a.view as string),
                wall: null,
                level: ids.l1,
                elevation: 0,
                target: IN_PLACE_ID,
                typeId: IN_PLACE_ID,
                curves: [],
                bad: [],
                error: null,
                canUndo: false,
                canRedo: false,
                form: {
                  kind:
                    kind === "Sweep"
                      ? {
                          Sweep: {
                            elevation: 0,
                            profile: { Rectangle: { width: 152.4, height: 152.4 } },
                          },
                        }
                      : kind === "Blend"
                        ? { Blend: { base: 0, top: 304.8, top_sketch: [] } }
                        : { Extrusion: { start: 0, end: 304.8 } },
                  void: kind === "VoidExtrusion",
                  index: (a.index as number | null) ?? null,
                  top: false,
                },
                region: null,
              },
            };
          }
          return fake.state;
        case "sketch_set_form":
          if (fake.state?.sketch?.form)
            fake.state = {
              ...fake.state,
              sketch: {
                ...fake.state.sketch,
                form: { ...fake.state.sketch.form, kind: a.kind as FormKind },
              },
            };
          return fake.state;
        case "in_place_delete_form":
          if (fake.state?.inPlace)
            fake.state = {
              ...fake.state,
              inPlace: {
                ...fake.state.inPlace,
                forms: fake.state.inPlace.forms.filter((_, i) => i !== a.index),
              },
            };
          return fake.state;
        case "in_place_finish":
          if (fake.state?.inPlace?.forms.length === 0)
            throw {
              message: "Add a form (Extrusion, Blend or Sweep) to the model, or Cancel Model.",
            };
          if (fake.state) fake.state = { ...fake.state, inPlace: null };
          return fake.state;
        case "in_place_cancel":
          if (fake.state) fake.state = { ...fake.state, inPlace: null, sketch: null };
          return fake.state;
        case "sketch_draw":
          if (fake.state?.sketch) {
            const p = a.pts as { x: number; y: number }[];
            fake.state = {
              ...fake.state,
              sketch: {
                ...fake.state.sketch,
                curves: [...fake.state.sketch.curves, { pts: p, locked: false, isLine: true }],
                canUndo: true,
              },
            };
          }
          return fake.state;
        case "sketch_finish":
          // An in-place form (ADR-068): added to the model's forms.
          if (fake.state?.sketch?.form && fake.state.inPlace && fake.state.sketch.curves.length) {
            const f = fake.state.sketch.form;
            const label =
              "Sweep" in f.kind
                ? "Sweep"
                : "Blend" in f.kind
                  ? "Blend"
                  : f.void
                    ? "Void Extrusion"
                    : "Extrusion";
            const n = fake.state.inPlace.forms.filter((l) => l.startsWith(label + " ")).length + 1;
            fake.state = {
              ...fake.state,
              sketch: null,
              inPlace: {
                ...fake.state.inPlace,
                forms: [...fake.state.inPlace.forms, `${label} ${n}`],
              },
            };
            return fake.state;
          }
          if (fake.state?.sketch) {
            fake.state =
              fake.state.sketch.curves.length === 0
                ? {
                    ...fake.state,
                    sketch: {
                      ...fake.state.sketch,
                      error:
                        "Lines must be in closed loops. The highlighted lines are open on one end.",
                    },
                  }
                : { ...fake.state, sketch: null };
          }
          return fake.state;
        case "sketch_cancel":
          if (fake.state) fake.state = { ...fake.state, sketch: null };
          return fake.state;
        case "sketch_preview":
          return [];
        case "parse_length":
          return a.text === '0"' ? 0 : 304.8;
        case "claude_key_set":
          return fake.claudeKey;
        case "claude_set_key":
          fake.claudeKey = !!(a.key as string).trim();
          return fake.claudeKey;
        case "plans_to_model":
          fake.plans = a.inputs;
          return {
            state: fake.state,
            summary: "A cantilevered house over a stream, on three floors.",
            report: {
              name: "Fallingwater",
              levels: 4,
              walls: 82,
              doors: 8,
              windows: 6,
              rooms: 32,
              floors: 8,
              roofs: 1,
              warnings: ["First Floor: a door at (490, 428) px has no wall near it"],
            },
          };
        case "generate_building":
          fake.generated = a.inputs;
          return {
            state: fake.state,
            summary: "A three-bedroom modern farmhouse of about 2,400 sf.",
            report: {
              name: "Oak Hollow",
              levels: 3,
              walls: 32,
              doors: 12,
              windows: 18,
              floors: 2,
              rooms: 14,
              stairs: 1,
              roofs: 1,
              materials: 3,
              warnings: ["Story 2: Loft has no route in"],
            },
          };
        case "material_library":
          return LIBRARY;
        case "building_types":
          return [
            { id: "SingleFamily", label: "Single-family house" },
            { id: "Hotel", label: "Hotel" },
          ];
        case "sheet_set_plan": {
          const o = a.options as { buildingType: string; phases: string[] };
          const sheet = (number: string, name: string, phases: string[], placeholder = false) => ({
            number,
            name,
            discipline: "Architectural",
            contents: placeholder
              ? "Placeholder by the structural engineer: footings"
              : 'Level 1 at 1/4" = 1\'-0"',
            placeholder,
            phases: phases.filter((p) => o.phases.includes(p)),
            exists: false,
          });
          const sheets = [
            sheet("G-001", "Cover Sheet & Sheet Index", ["PD", "SD", "DD", "CD", "BN", "CA"]),
            sheet("A-101", "Level 1 Floor Plan", ["SD", "DD", "CD", "BN", "CA"]),
            sheet(
              "S-101",
              o.buildingType === "Hotel" ? "Hotel Foundation Plan" : "Foundation Plan",
              ["DD", "CD", "BN", "CA"],
              true,
            ),
          ].filter((s) => s.phases.length);
          const names: Record<string, [string, string][]> = {
            SD: [["sd100", "100% Schematic Design"]],
            CD: [
              ["cd100", "100% Construction Documents"],
              ["permit", "Permit Set"],
            ],
          };
          return {
            deliverables: o.phases.flatMap((p) =>
              (names[p] ?? []).map(([id, name]) => ({
                phase: p,
                stage: p,
                id,
                name,
                sheets: sheets.filter((s) => s.phases.includes(p)).map((s) => s.number),
              })),
            ),
            sheets,
            warnings: [],
          };
        }
        case "create_sheet_sets":
          return {
            state: fake.state,
            report: {
              created: 3,
              updated: 0,
              views: 1,
              sections: 2,
              placeholders: 1,
              warnings: [],
            },
          };
        case "export_sheet_sets":
          return {
            state: fake.state,
            files: [
              { name: "Permit Set", path: `${a.folder as string}/Permit Set.pdf`, sheets: 3 },
            ],
          };
        case "window_library":
          return WINDOW_LIBRARY;
        case "door_library":
          return DOOR_LIBRARY;
        case "door_preview": {
          const s = a.spec as { width: number; height: number; leaf: string };
          return {
            name: `Door ${s.leaf} ${Math.round(s.width / 25.4)}"`,
            preview: thumb(s.width, s.height),
          };
        }
        case "load_door_types": {
          const specs = a.specs as unknown[];
          const loaded = specs.map((_, i) => ({
            id: `00000000-0000-7000-8000-0000000000d${i}`,
            name: `Loaded door ${i + 1}`,
          }));
          if (fake.state)
            fake.state = { ...fake.state, doorTypes: [...fake.state.doorTypes, ...loaded] };
          return { state: fake.state, ids: loaded.map((t) => t.id) };
        }
        case "opening_thumbnail":
          return { frame: [], glass: [], wall: [], color: [240, 240, 236] };
        case "window_preview": {
          const s = a.spec as { width: number; height: number; grille: string; finish: string };
          const extra = [s.grille !== "None" && s.grille, s.finish !== "White" && s.finish]
            .filter(Boolean)
            .join(", ");
          return {
            name: `Window ${Math.round(s.width / 25.4)}" x ${Math.round(s.height / 25.4)}"${extra ? ` - ${extra}` : ""}`,
            preview: thumb(s.width, s.height),
          };
        }
        case "load_window_types": {
          const specs = a.specs as unknown[];
          const loaded = specs.map((_, i) => ({
            id: `00000000-0000-7000-8000-0000000000b${i}`,
            name: `Loaded window ${i + 1}`,
          }));
          if (fake.state)
            fake.state = { ...fake.state, windowTypes: [...fake.state.windowTypes, ...loaded] };
          return { state: fake.state, ids: loaded.map((t) => t.id) };
        }
        case "render_materials":
          return (fake.state?.materials ?? []).map((m) => ({
            id: m.id,
            name: m.name,
            color: [200, 200, 200],
            appearance: m.name.startsWith("White Oak")
              ? LIBRARY[0]!.appearance
              : { ...LIBRARY[1]!.appearance, preset: null },
          }));
        case "add_library_material": {
          const p = LIBRARY.find((x) => x.id === a.id)!;
          if (fake.state)
            fake.state = {
              ...fake.state,
              materials: [
                ...fake.state.materials,
                { id: "00000000-0000-7000-8000-0000000000a1", name: p.name },
              ],
            };
          return fake.state;
        }
        case "paint_elements":
          return fake.state;
        case "apply_material":
          fake.applied.push([a.ids as string[], a.material as string]);
          return fake.state;
        case "create_material":
          if (fake.state)
            fake.state = {
              ...fake.state,
              materials: [
                ...fake.state.materials,
                { id: "00000000-0000-7000-8000-000000000052", name: "Brick 2" },
              ],
            };
          return fake.state;
        case "drawing_options":
          return [
            [
              ["Centerline", "Wall Centerline"],
              ["FinishExterior", "Finish Face: Exterior"],
            ],
            [
              ["straight", "Straight"],
              ["l-left", "L-Shaped, Turning Left"],
            ],
          ];
        case "add_project_parameter":
          if (fake.state)
            fake.state = {
              ...fake.state,
              paramDefs: [
                ...fake.state.paramDefs,
                {
                  key: String(a.label).toLowerCase().replace(/\W+/g, "_"),
                  label: a.label as string,
                  kind: a.kind as AppState["paramDefs"][number]["kind"],
                  scope: a.typeScope ? "Type" : "Instance",
                  categories: a.categories as AppState["paramDefs"][number]["categories"],
                },
              ],
            };
          return fake.state;
        case "remove_project_parameter":
          if (fake.state)
            fake.state = {
              ...fake.state,
              paramDefs: fake.state.paramDefs.filter((d) => d.key !== a.key),
            };
          return fake.state;
        case "cloud_status":
          return { configured: true, signedIn: false, email: null, name: null };
        case "cloud_sign_in":
          return { configured: true, signedIn: true, email: a.email, name: "Ada Arch" };
        case "cloud_projects":
          return [
            { id: "p1", name: "Lake House", slug: "lake-house" },
            { id: "p2", name: "Studio Loft", slug: "studio-loft" },
          ];
        case "link_rufplan":
          if (fake.state) fake.state = { ...fake.state, rufplan: a.link as AppState["rufplan"] };
          return fake.state;
        case "publish_options":
          return {
            phaseKind: "sd",
            stage: "SD",
            sheetCount: 4,
            deliverables: [
              { id: "sd30", label: "30% Schematic Design" },
              { id: "sd60", label: "60% Schematic Design" },
            ],
          };
        case "publish_to_rufplan":
          return {
            state: fake.state,
            url: "https://rufplan.io/projects/lake-house",
            published: ["New Project - SD Set.pdf"],
            skipped: ["New Project - SD Set.ifc: mime type application/x-step is not supported"],
          };
        case "set_pinned":
          fake.pinned = a.pinned ? [...(a.ids as string[])] : [];
          return fake.state;
        case "select_all_instances":
          return ["w1", "w2", "w3"];
        // Box selection and Filter: two walls and a door in the box.
        case "pick_in_rect":
          return ["w1", "w2", "d1"];
        case "element_categories": {
          const known: Record<string, string> = { w1: "Wall", w2: "Wall", w3: "Wall", d1: "Door" };
          return (a.ids as string[])
            .map((id) => [id, fake.properties?.[id]?.category ?? known[id]])
            .filter((x) => x[1]);
        }
        case "selection_categories": {
          // From the categories the test gave, else the view's (w… walls, d… doors).
          const known: Record<string, string> = {
            w1: "Wall",
            w2: "Wall",
            w3: "Wall",
            d1: "Door",
            [IN_PLACE_ID]: "Casework",
          };
          const cats = (a.ids as string[])
            .map((id) => fake.properties?.[id]?.category ?? known[id])
            .filter(Boolean);
          return [...new Set(cats)];
        }
        case "snap":
          return { pt: a.point, kind: "None", label: null };
        // Detail components (ADR-071).
        case "detail_component_types":
          return [
            {
              key: "lum-2x6",
              family: "CutLumber",
              familyLabel: "Nominal Cut Lumber-Section",
              name: "2x6",
              lineBased: false,
            },
            {
              key: "brick-mod",
              family: "BrickCoursing",
              familyLabel: "Brick-Standard-Section (Repeating)",
              name: 'Modular (3 5/8")',
              lineBased: true,
            },
            {
              key: "batt-55",
              family: "BattInsulation",
              familyLabel: "Insulation-Batt",
              name: '5 1/2" Batt',
              lineBased: true,
            },
          ];
        case "detail_component_preview":
          return [[a.start, a.end]];
        case "create_detail_component":
          if (fake.state) fake.state = { ...fake.state, revision: fake.state.revision + 1 };
          return fake.state;
        // Revit's Text (ADR-070).
        case "create_text_note":
        case "add_text_leader":
        case "remove_text_leader":
          if (fake.state) fake.state = { ...fake.state, revision: fake.state.revision + 1 };
          return fake.state;
        case "text_note_info":
          return {
            text: "EXISTING NOTE",
            at: { x: 1000, y: 1000 },
            size: 2.4,
            align: "Left",
            width: null,
            view: ids.plan1,
            min: { x: 1000, y: 950 },
            max: { x: 2000, y: 1050 },
          };
        // Lighting (ADR-057).
        case "lighting_library":
          return FAKE_LIGHT_LIBRARY;
        case "load_lighting_types":
          return { state: fake.state, ids: ["00000000-0000-7000-8000-000000000044"] };
        case "fixture_thumbnail":
          return { body: [], lens: [], color: [236, 236, 232], glow: [255, 230, 200] };
        case "lights":
          return fake.lights;
        case "set_lights":
        case "set_sun_settings":
        case "create_lighting_fixture":
          return fake.state;
        // Vegetation (ADR-064).
        case "planting_library":
          return FAKE_PLANT_LIBRARY;
        case "load_planting_types":
          return { state: fake.state, ids: ["00000000-0000-7000-8000-000000000061"] };
        case "plant_instances":
          return [];
        case "ground_library":
          return [
            {
              id: "site-lawn",
              name: "Lawn, Manicured",
              description: "Fine mown turf.",
              color: [92, 124, 60],
              texture: "gen:lawn",
            },
          ];
        case "create_plants":
        case "set_base_ground":
        case "paint_grass":
        case "erase_grass":
          return fake.state;
        // Grass Brush (ADR-065).
        case "grass_kinds":
          return [
            {
              kind: "Lawn",
              label: "Lawn",
              spec: { kind: "Lawn", height: 70, variation: 0.35, density: 1, color: [86, 124, 54] },
            },
          ];
        case "grass_patches":
          return [];
        case "sun_for":
        case "sun_now":
          return { dir: [0.5, -0.5, 0.7], altitude: 35, azimuth: 225 };
        case "pick_cycle":
          return fake.underCursor;
        case "pick":
          return fake.underCursor[0]?.ids[0] ?? null;
        case "tag_elements":
        case "move_elements":
          return fake.state;
        case "view_categories":
          return [
            ["w1", "Wall"],
            ["d1", "Door"],
          ];
        case "plugin:dialog|open":
          return fake.openPath;
        case "plugin:dialog|save":
          return fake.savePath;
      }
    },
    { shouldMockEvents: true },
  );
  return fake;
}

export const commandsCalled = (fake: FakeBackend) =>
  fake.calls
    .map((c) => c.cmd)
    .filter(
      (c) => !c.startsWith("plugin:event") && c !== "view_display_list" && c !== "properties",
    );

const fixtureSpec = (over: Partial<import("../bindings/FixtureSpec").FixtureSpec>) => ({
  family: "Downlight" as const,
  mount: "Ceiling" as const,
  width: 190.5,
  depth: 190.5,
  height: 88.9,
  drop: 0,
  mount_height: 0,
  lumens: 1200,
  watts: 14,
  kelvin: 3000,
  shape: "Circle" as const,
  distribution: "Spot" as const,
  beam: 70,
  ...over,
});

/** A small lighting library: a downlight, a troffer and a bollard. */
export const FAKE_LIGHT_LIBRARY: import("../bindings/LightLibrary").LightLibrary = {
  groups: ["Recessed & Ceiling", "Site & Exterior"],
  uses: [
    { id: "Residential", label: "Residential" },
    { id: "Office", label: "Office" },
    { id: "Exterior", label: "Site & Exterior" },
  ],
  presets: [
    {
      name: '6" LED Downlight',
      description: 'Recessed 6" can',
      group: "Recessed & Ceiling",
      uses: ["Residential", "Office"],
      spec: fixtureSpec({}),
    },
    {
      name: "2x4 LED Troffer",
      description: "Lay-in 2'x4' troffer",
      group: "Recessed & Ceiling",
      uses: ["Office"],
      spec: fixtureSpec({
        family: "Troffer",
        width: 609.6,
        depth: 1219.2,
        lumens: 4000,
        watts: 32,
        kelvin: 4000,
        shape: "Rectangle",
        distribution: "Hemispherical",
        beam: 180,
      }),
    },
    {
      name: '42" Bollard',
      description: "Path bollard",
      group: "Site & Exterior",
      uses: ["Exterior"],
      spec: fixtureSpec({
        family: "Bollard",
        mount: "Ground",
        height: 1066.8,
        lumens: 900,
        distribution: "Hemispherical",
        shape: "Point",
      }),
    },
  ],
};
