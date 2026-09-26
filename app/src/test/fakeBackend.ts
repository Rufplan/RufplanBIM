import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppState } from "../ipc";

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
  /** Categories to report for selected ids (anything else is a view). */
  properties?: Record<string, { category: string }>;
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
    site: false,
    camera: null as AppState["views"][number]["camera"],
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
    materials: [
      { id: "00000000-0000-7000-8000-000000000050", name: "Brick" },
      { id: "00000000-0000-7000-8000-000000000051", name: "Concrete" },
    ],
    paramDefs: [],
  };
}

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
  };

  mockIPC(
    (cmd, args) => {
      fake.calls.push({ cmd, args });
      if (fake.failWith && !cmd.startsWith("plugin:")) throw { message: fake.failWith };
      const a = args as Record<string, unknown>;
      switch (cmd) {
        case "app_state":
          return fake.state;
        case "project_new":
          return (fake.state = appState(null));
        case "project_open":
          return (fake.state = appState(a.path as string));
        case "project_save":
          return (fake.state = appState(
            (a.path as string | null) ?? fake.state?.project.path ?? null,
          ));
        case "set_property":
          if (fake.state && a.key === "current_stage")
            fake.state = { ...fake.state, currentStage: a.value as string };
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
                kind: a.kind as "Floor" | "Ceiling",
                // From 3D, the sketch goes through the level's plan (ADR-025).
                view: a.view === ids.v3d ? ids.plan1 : (a.view as string),
                level: (a.level as string | null) ?? ids.l1,
                elevation: 0,
                target: (a.target as string | null) ?? null,
                typeId: (a.typeId as string | null) ?? ids.ft,
                curves: [],
                bad: [],
                error: null,
                canUndo: false,
                canRedo: false,
              },
            };
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
