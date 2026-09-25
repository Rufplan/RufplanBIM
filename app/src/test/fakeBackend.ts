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

/** Installs an in-memory stand-in for the Rust commands and dialog plugin. */
export function installFakeBackend(): FakeBackend {
  const fake: FakeBackend = {
    calls: [],
    openPath: null,
    savePath: null,
    failWith: null,
    state: null,
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
          return { id: a.id, category: "View", title: "Level 1", typeId: null, properties: [] };
        case "handles":
          return { grips: [], dims: [] };
        case "sketch_begin":
          if (fake.state)
            fake.state = {
              ...fake.state,
              sketch: {
                kind: a.kind as "Floor" | "Ceiling",
                view: a.view as string,
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
