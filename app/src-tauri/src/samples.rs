//! The Modern House sample (ADR-093): a furnished two-story modern house built from
//! everything the model has. A white stucco base with a garage wing, a guest and office wing
//! and an open living, dining and kitchen behind a folding glass wall; a flat-roofed cedar
//! upper volume that cantilevers 4' over the south terrace on two steel columns; a straight
//! stair under an open gallery; furniture, appliances, lighting, landscape and a drawing set.
//!
//! Plans are laid out in feet (y north). Exterior walls run clockwise so their finish faces
//! the outside; doors, windows, furniture and fixtures are placed by their center point.

use std::f64::consts::{FRAC_PI_2, PI};

use anyhow::{bail, Context};
use studio_core::doors::{DoorSpec, LeafStyle};
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::windows::{FrameFinish, Grille, WindowSpec};
use studio_core::{ops, Category, Document, ElementData, ElementId, ViewKind};
use studio_core::{DoorFamily, LayerFunction, WallFunction, WallLayer, WindowFamily};
use studio_geom::Pt;

/// A point in feet.
fn ft(x: f64, y: f64) -> Pt {
    Pt::new(x * MM_PER_FT, y * MM_PER_FT)
}

/// The rotation that turns a piece of furniture's front (its local -y) toward `deg`
/// (0 = east, 90 = north, 180 = west, 270 = south).
fn facing(deg: f64) -> f64 {
    deg.to_radians() + FRAC_PI_2
}
const NORTH: f64 = 90.0;
const SOUTH: f64 = 270.0;
const EAST: f64 = 0.0;
const WEST: f64 = 180.0;
/// The Modern House hero rendering (ADR-095), 1600 x 900, rendered with the dev aid
/// `--autorender` from its Hero View camera.
const HERO_RENDERING: &[u8] = include_bytes!("../samples/hero-rendering.jpg");

/// The L1 outline (counter-clockwise, feet).
const L1: &[(f64, f64)] = &[
    (0.0, 8.0),
    (24.0, 8.0),
    (24.0, 0.0),
    (66.0, 0.0),
    (66.0, 26.0),
    (58.0, 26.0),
    (58.0, 40.0),
    (30.0, 40.0),
    (30.0, 32.0),
    (0.0, 32.0),
];

/// The L2 outline (counter-clockwise): the upper volume, 4' past the base to the south.
const L2: &[(f64, f64)] = &[
    (26.0, -4.0),
    (62.0, -4.0),
    (62.0, 12.0),
    (66.0, 12.0),
    (66.0, 34.0),
    (26.0, 34.0),
];

/// A wall from one point to another (feet).
type Run = ((f64, f64), (f64, f64));

struct B<'a> {
    doc: &'a mut Document,
}

impl B<'_> {
    fn named(&self, cat: Category, prefix: &str) -> anyhow::Result<ElementId> {
        self.doc
            .of(cat)
            .find(|e| e.data.name().starts_with(prefix))
            .map(|e| e.id)
            .with_context(|| format!("missing {prefix}"))
    }

    fn wall_type(&mut self, name: &str, layers: Vec<WallLayer>) -> anyhow::Result<ElementId> {
        let thickness = layers.iter().map(|l| l.thickness).sum();
        let function = if name.starts_with("Exterior") {
            WallFunction::Exterior
        } else {
            WallFunction::Interior
        };
        Ok(self.doc.transact("Add wall type", |tx| {
            Ok(tx.insert(ElementData::WallType {
                name: name.into(),
                thickness,
                function,
                layers,
            }))
        })?)
    }

    /// Exterior walls around an outline given counter-clockwise, run clockwise.
    fn outline(
        &mut self,
        wt: ElementId,
        level: ElementId,
        pts: &[(f64, f64)],
    ) -> anyhow::Result<Vec<ElementId>> {
        let cw: Vec<Pt> = pts.iter().rev().map(|(x, y)| ft(*x, *y)).collect();
        let mut out = vec![];
        for i in 0..cw.len() {
            out.push(ops::create_wall(
                self.doc,
                wt,
                level,
                cw[i],
                cw[(i + 1) % cw.len()],
            )?);
        }
        Ok(out)
    }

    fn walls(&mut self, wt: ElementId, level: ElementId, runs: &[Run]) -> anyhow::Result<()> {
        for (a, b) in runs {
            ops::create_wall(self.doc, wt, level, ft(a.0, a.1), ft(b.0, b.1))?;
        }
        Ok(())
    }

    /// The wall on `level` whose centerline passes through `at`, and how far along it.
    fn host(&self, level: ElementId, at: Pt) -> anyhow::Result<(ElementId, f64, Pt, Pt)> {
        for e in self.doc.of(Category::Wall) {
            let ElementData::Wall {
                start,
                end,
                base_level,
                ..
            } = &e.data
            else {
                continue;
            };
            if *base_level != level {
                continue;
            }
            let len = start.dist(*end);
            let d = end.sub(*start).scale(1.0 / len);
            let t = at.sub(*start).dot(d);
            let off = at.sub(*start).dot(d.perp());
            if off.abs() < 1.0 && t > 0.0 && t < len {
                return Ok((e.id, t, *start, *end));
            }
        }
        bail!(
            "no wall at ({:.1}, {:.1})",
            at.x / MM_PER_FT,
            at.y / MM_PER_FT
        )
    }

    /// A door centered at `at` (feet) swinging toward `into`.
    fn door(
        &mut self,
        ty: ElementId,
        level: ElementId,
        at: (f64, f64),
        into: (f64, f64),
    ) -> anyhow::Result<ElementId> {
        let p = ft(at.0, at.1);
        let (wall, t, s, e) = self.host(level, p)?;
        let flip = ft(into.0, into.1).sub(s).dot(e.sub(s).perp()) < 0.0;
        Ok(ops::create_door(self.doc, ty, wall, t, flip)?)
    }

    /// A window centered at `at` (feet), its exterior toward `out`.
    fn window(
        &mut self,
        ty: ElementId,
        level: ElementId,
        at: (f64, f64),
        out: (f64, f64),
    ) -> anyhow::Result<ElementId> {
        let p = ft(at.0, at.1);
        let (wall, t, s, e) = self.host(level, p)?;
        let flip = ft(out.0, out.1).sub(s).dot(e.sub(s).perp()) < 0.0;
        Ok(ops::create_window(self.doc, ty, wall, t, flip)?)
    }

    fn ffe(&mut self, name: &str) -> anyhow::Result<ElementId> {
        Ok(studio_core::ffe::load(self.doc, &[name.to_string()])?[0])
    }

    /// Furniture or equipment at `at` (feet), its front toward `deg`.
    fn put(
        &mut self,
        name: &str,
        level: ElementId,
        at: (f64, f64),
        deg: f64,
    ) -> anyhow::Result<()> {
        let t = self.ffe(name)?;
        studio_core::ffe::create(self.doc, t, level, ft(at.0, at.1), facing(deg))?;
        Ok(())
    }

    fn light(
        &mut self,
        name: &str,
        level: ElementId,
        at: &[(f64, f64)],
        rot: f64,
        elevation: Option<f64>,
    ) -> anyhow::Result<()> {
        let t = studio_core::lighting::load(self.doc, &[name.to_string()])?[0];
        for p in at {
            studio_core::lighting::create_fixture(
                self.doc,
                t,
                level,
                ft(p.0, p.1),
                rot,
                elevation,
            )?;
        }
        Ok(())
    }

    fn plants(
        &mut self,
        name: &str,
        level: ElementId,
        at: &[(f64, f64)],
        scale: f64,
    ) -> anyhow::Result<()> {
        let t = studio_core::planting::load(self.doc, &[name.to_string()])?[0];
        let pts: Vec<(Pt, f64, f64)> = at
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // A little variety, the same every time.
                let k = (i as f64 * 2.399).sin();
                (ft(p.0, p.1), i as f64 * 1.7, scale * (1.0 + 0.08 * k))
            })
            .collect();
        studio_core::planting::create_plants(self.doc, t, level, &pts)?;
        Ok(())
    }

    fn room(&mut self, level: ElementId, at: (f64, f64), name: &str) -> anyhow::Result<()> {
        let r = ops::create_room(self.doc, level, ft(at.0, at.1))
            .with_context(|| format!("room {name}"))?;
        ops::set_property(self.doc, r, "name", name, 0)?;
        Ok(())
    }

    /// Casework modelled in place (ADR-068): boxes from z0 to z1 (feet), cut by voids.
    fn casework(
        &mut self,
        name: &str,
        level: ElementId,
        solids: &[(f64, f64, f64, f64, f64, f64)],
        voids: &[(f64, f64, f64, f64, f64, f64)],
    ) -> anyhow::Result<()> {
        use studio_core::inplace::{add_form, create, Form, FormKind};
        use studio_core::sketch::SketchCurve;
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| -> Vec<Vec<SketchCurve>> {
            let p = [ft(x0, y0), ft(x1, y0), ft(x1, y1), ft(x0, y1)];
            vec![(0..4)
                .map(|i| SketchCurve::line(p[i], p[(i + 1) % 4]))
                .collect()]
        };
        let id = create(self.doc, Category::Casework, Some(name), level)?;
        for (void, list) in [(false, solids), (true, voids)] {
            for (x0, y0, x1, y1, z0, z1) in list {
                add_form(
                    self.doc,
                    id,
                    Form {
                        kind: FormKind::Extrusion {
                            start: z0 * MM_PER_FT,
                            end: z1 * MM_PER_FT,
                        },
                        sketch: rect(*x0, *y0, *x1, *y1),
                        void,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn ground(&mut self, level: ElementId, preset: &str, pts: &[(f64, f64)]) -> anyhow::Result<()> {
        let material = studio_core::library::add_preset(self.doc, preset)?;
        let boundary: Vec<Pt> = pts.iter().map(|(x, y)| ft(*x, *y)).collect();
        self.doc.transact("Create ground region", |tx| {
            Ok(tx.insert(ElementData::GroundRegion {
                level,
                material,
                boundary,
                sketch: vec![],
            }))
        })?;
        Ok(())
    }

    fn view_where(&self, pred: &dyn Fn(&ViewKind, &str) -> bool) -> anyhow::Result<ElementId> {
        self.doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind, name, .. } if pred(kind, name)))
            .map(|e| e.id)
            .context("missing sample view")
    }
}

fn layer(name: &str, inches: f64, function: LayerFunction) -> WallLayer {
    WallLayer {
        name: name.into(),
        thickness: inches * MM_PER_IN,
        function,
        material: None,
    }
}

fn door_spec(family: DoorFamily, leaf: LeafStyle, panels: u32, w: f64, h: f64) -> DoorSpec {
    DoorSpec {
        family,
        leaf,
        panels,
        width: w * MM_PER_IN,
        height: h * MM_PER_IN,
        finish: None,
    }
}

fn window_spec(family: WindowFamily, units: u32, w: f64, h: f64, sill: f64) -> WindowSpec {
    WindowSpec {
        family,
        units,
        width: w * MM_PER_IN,
        height: h * MM_PER_IN,
        sill: sill * MM_PER_IN,
        grille: Grille::None,
        finish: FrameFinish::Black,
    }
}

pub fn build_modern(doc: &mut Document) -> anyhow::Result<()> {
    let levels = doc.levels();
    let (l1, l2) = (levels[0].0, levels[1].0);
    let mut b = B { doc };
    let partition = b.named(Category::WallType, "Interior - 4 7/8")?;
    let interior = b.named(Category::WallType, "Interior - 6")?;
    use LayerFunction::*;
    let stucco = b.wall_type(
        "Exterior - Stucco on 2x6 Stud",
        vec![
            layer("Stucco", 0.875, Finish),
            layer("Plywood Sheathing", 0.5, Substrate),
            layer("Wood Stud 2x6 with Batt Insulation", 5.5, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ],
    )?;
    let cedar = b.wall_type(
        "Exterior - Cedar Rainscreen on 2x6 Stud",
        vec![
            layer("Vertical Cedar Siding on Furring", 1.5, Finish),
            layer("Plywood Sheathing", 0.5, Substrate),
            layer("Wood Stud 2x6 with Batt Insulation", 5.5, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ],
    )?;

    // ---------------------------------------------------------------- walls
    let base = b.outline(stucco, l1, L1)?;
    b.walls(interior, l1, &[((24.0, 8.0), (24.0, 32.0))])?;
    b.walls(
        partition,
        l1,
        &[
            // The living room's TV wall, standing free at its east end.
            ((24.0, 18.0), (42.0, 18.0)),
            // Mudroom and powder room.
            ((30.0, 18.0), (30.0, 26.0)),
            ((30.0, 26.0), (30.0, 32.0)),
            ((24.0, 26.0), (30.0, 26.0)),
            // The guest and office wing off the stair hall.
            ((30.0, 26.0), (58.0, 26.0)),
            ((44.0, 26.0), (44.0, 40.0)),
            ((50.0, 26.0), (50.0, 40.0)),
        ],
    )?;
    let upper = b.outline(cedar, l2, L2)?;
    b.walls(
        partition,
        l2,
        &[
            // The primary suite: bedroom, bath and walk-in closet.
            ((46.0, -4.0), (46.0, 12.0)),
            ((46.0, 4.0), (62.0, 4.0)),
            ((26.0, 12.0), (62.0, 12.0)),
            // Bedroom 2, and the rooms off the gallery.
            ((54.0, 12.0), (54.0, 34.0)),
            ((26.0, 25.0), (54.0, 25.0)),
            ((40.0, 25.0), (40.0, 34.0)),
            ((47.0, 25.0), (47.0, 34.0)),
        ],
    )?;
    // Open plan: room separators between living, dining, kitchen, stair hall and foyer.
    for (a, c) in [
        ((44.0, 0.0), (44.0, 18.0)),
        ((54.0, 0.0), (54.0, 18.0)),
        ((42.0, 18.0), (66.0, 18.0)),
        ((58.0, 18.0), (58.0, 26.0)),
    ] {
        studio_core::detail::create_room_separator(b.doc, l1, ft(a.0, a.1), ft(c.0, c.1))?;
    }

    // ---------------------------------------------------------------- doors and windows
    let entry = studio_core::doors::load(
        b.doc,
        &[DoorSpec {
            finish: Some(studio_core::doors::DoorFinish::Walnut),
            ..door_spec(DoorFamily::SingleFlush, LeafStyle::Flush, 0, 42.0, 96.0)
        }],
    )?[0];
    let flush36 = b.named(Category::DoorType, "Single Flush 36")?;
    let flush30 = b.named(Category::DoorType, "Single Flush 30")?;
    let doors = studio_core::doors::load(
        b.doc,
        &[
            door_spec(DoorFamily::Garage, LeafStyle::Flush, 1, 192.0, 84.0),
            door_spec(DoorFamily::FoldingWall, LeafStyle::FullLite, 4, 144.0, 96.0),
            door_spec(DoorFamily::SlidingGlass, LeafStyle::FullLite, 2, 72.0, 96.0),
            door_spec(DoorFamily::Pocket, LeafStyle::Flush, 1, 30.0, 80.0),
            door_spec(DoorFamily::SingleFlush, LeafStyle::FullLite, 0, 36.0, 80.0),
        ],
    )?;
    let (garage, folding, sliding, pocket, glass36) =
        (doors[0], doors[1], doors[2], doors[3], doors[4]);
    let windows = studio_core::windows::load(
        b.doc,
        &[
            window_spec(WindowFamily::Fixed, 1, 72.0, 60.0, 24.0),
            window_spec(WindowFamily::Casement, 1, 36.0, 48.0, 36.0),
            window_spec(WindowFamily::Casement, 2, 60.0, 48.0, 42.0),
            window_spec(WindowFamily::Awning, 1, 36.0, 24.0, 60.0),
            window_spec(WindowFamily::Slider, 1, 60.0, 48.0, 36.0),
            window_spec(WindowFamily::PictureCasement, 1, 96.0, 60.0, 24.0),
            window_spec(WindowFamily::Storefront, 1, 120.0, 96.0, 6.0),
            window_spec(WindowFamily::Fixed, 1, 36.0, 18.0, 66.0),
        ],
    )?;
    let (fixed72, case36, case60, awning, slider, picture, glazing, clerestory) = (
        windows[0], windows[1], windows[2], windows[3], windows[4], windows[5], windows[6],
        windows[7],
    );
    // Level 1.
    b.door(entry, l1, (62.0, 26.0), (62.0, 22.0))?;
    b.door(garage, l1, (0.0, 20.0), (6.0, 20.0))?;
    b.door(flush36, l1, (24.0, 22.0), (27.0, 22.0))?;
    b.door(pocket, l1, (30.0, 23.5), (33.0, 23.5))?;
    b.door(flush30, l1, (27.0, 26.0), (27.0, 29.0))?;
    b.door(flush36, l1, (37.0, 26.0), (37.0, 30.0))?;
    b.door(flush30, l1, (47.0, 26.0), (47.0, 30.0))?;
    b.door(glass36, l1, (54.0, 26.0), (54.0, 30.0))?;
    b.door(folding, l1, (34.0, 0.0), (34.0, -4.0))?;
    b.door(sliding, l1, (49.0, 0.0), (49.0, -4.0))?;
    for (ty, at, out) in [
        (case60, (60.75, 0.0), (60.75, -4.0)),
        (case36, (66.0, 22.0), (70.0, 22.0)),
        (clerestory, (66.0, 5.0), (70.0, 5.0)),
        (slider, (12.0, 8.0), (12.0, 4.0)),
        (awning, (12.0, 32.0), (12.0, 36.0)),
        (awning, (27.0, 32.0), (27.0, 36.0)),
        (slider, (37.0, 40.0), (37.0, 44.0)),
        (case36, (30.0, 36.0), (26.0, 36.0)),
        (awning, (47.0, 40.0), (47.0, 44.0)),
        (fixed72, (54.0, 40.0), (54.0, 44.0)),
        (case36, (58.0, 33.0), (62.0, 33.0)),
    ] {
        b.window(ty, l1, at, out)?;
    }
    // Level 2.
    b.door(flush36, l2, (30.0, 12.0), (30.0, 8.0))?;
    b.door(flush30, l2, (46.0, 9.0), (49.0, 9.0))?;
    b.door(flush30, l2, (46.0, 1.5), (49.0, 1.5))?;
    b.door(pocket, l2, (50.0, 4.0), (50.0, 1.0))?;
    b.door(flush36, l2, (54.0, 15.0), (57.0, 15.0))?;
    b.door(flush36, l2, (36.0, 25.0), (36.0, 28.0))?;
    b.door(flush30, l2, (43.5, 25.0), (43.5, 28.0))?;
    b.door(flush30, l2, (50.5, 25.0), (50.5, 28.0))?;
    for (ty, at, out) in [
        (picture, (36.0, -4.0), (36.0, -8.0)),
        (case36, (26.0, 5.0), (22.0, 5.0)),
        (awning, (50.0, -4.0), (50.0, -8.0)),
        (awning, (58.0, -4.0), (58.0, -8.0)),
        (case36, (62.0, 0.0), (66.0, 0.0)),
        (clerestory, (62.0, 8.0), (66.0, 8.0)),
        (fixed72, (66.0, 20.0), (70.0, 20.0)),
        (case36, (66.0, 29.0), (70.0, 29.0)),
        (slider, (60.0, 34.0), (60.0, 38.0)),
        (slider, (33.0, 34.0), (33.0, 38.0)),
        (awning, (43.5, 34.0), (43.5, 38.0)),
        (awning, (50.5, 34.0), (50.5, 38.0)),
        // Floor-to-ceiling glass at the top of the stair.
        (glazing, (26.0, 18.5), (22.0, 18.5)),
    ] {
        b.window(ty, l2, at, out)?;
    }

    // ---------------------------------------------------------------- floors, stair, roofs
    let slab = b.named(Category::FloorType, "Concrete Slab")?;
    let joists = b.named(Category::FloorType, "Wood Joist")?;
    studio_regen::derived::create_floor_by_walls(b.doc, slab, l1)?;
    studio_regen::derived::create_floor_by_walls(b.doc, joists, l2)?;
    // The stair climbs west along the stair hall, under the open gallery.
    studio_core::build::create_stair(
        b.doc,
        l1,
        ft(47.0, 20.0),
        ft(31.0, 20.0),
        studio_core::build::DEFAULT_STAIR_WIDTH,
    )?;
    let guard = b.named(Category::RailingType, "Guardrail")?;
    studio_core::structure::create_railing(
        b.doc,
        guard,
        l2,
        vec![
            ft(31.4, 18.1),
            ft(47.1, 18.1),
            ft(47.1, 21.9),
            ft(31.4, 21.9),
        ],
    )?;
    // Two steel columns carry the cantilevered corners.
    let column = b.named(Category::ColumnType, "Steel")?;
    for x in [27.0, 61.0] {
        studio_core::structure::create_column(b.doc, column, l1, ft(x, -3.5), 0.0)?;
    }
    // Flat roofs: the upper volume on a Roof level, the low wings at Level 2.
    let roof_level = ops::create_level(b.doc, 20.0 * MM_PER_FT)?;
    ops::set_property(b.doc, roof_level, "name", "Roof", 0)?;
    for v in b
        .doc
        .of(Category::View)
        .filter(|e| e.data.level() == Some(roof_level))
        .map(|e| e.id)
        .collect::<Vec<_>>()
    {
        ops::set_property(b.doc, v, "name", "Roof", 0)?;
    }
    let flat = b.named(Category::RoofType, "Flat Membrane")?;
    let ring = |pts: &[(f64, f64)], overhang: f64| {
        studio_geom::offset_ring(
            &pts.iter().map(|(x, y)| ft(*x, *y)).collect::<Vec<_>>(),
            overhang * MM_PER_FT,
        )
    };
    studio_core::build::create_roof(b.doc, flat, roof_level, 0.0, ring(L2, 2.0), 0.0)?;
    // The low roofs overhang 18" outside and stop at the upper volume's outer face.
    let face = 26.0 - 3.75 / 12.0;
    for pts in [
        &[
            (-1.5, 6.5),
            (22.5, 6.5),
            (22.5, -1.5),
            (face, -1.5),
            (face, 33.5),
            (-1.5, 33.5),
        ][..],
        &[(28.5, 34.32), (59.5, 34.32), (59.5, 41.5), (28.5, 41.5)][..],
        &[(62.32, -1.5), (67.5, -1.5), (67.5, 11.68), (62.32, 11.68)][..],
    ] {
        let boundary = pts.iter().map(|(x, y)| ft(*x, *y)).collect();
        studio_core::build::create_roof(b.doc, flat, l2, 0.0, boundary, 0.0)?;
    }

    // The deep white stepped band of a modern flat roof (ADR-095), on every roof.
    let all_roofs: Vec<ElementId> = b.doc.of(Category::Roof).map(|e| e.id).collect();
    studio_core::fascia::set(b.doc, &all_roofs, Some("Modern Stepped Band 12\""))?;

    // ---------------------------------------------------------------- rooms and ceilings
    for (level, at, name) in [
        (l1, (34.0, 9.0), "Living"),
        (l1, (49.0, 9.0), "Dining"),
        (l1, (60.0, 12.0), "Kitchen"),
        (l1, (40.0, 24.5), "Stair Hall"),
        (l1, (62.0, 22.0), "Foyer"),
        (l1, (27.0, 20.0), "Mudroom"),
        (l1, (27.0, 29.0), "Powder"),
        (l1, (12.0, 20.0), "Garage"),
        (l1, (37.0, 33.0), "Guest Bedroom"),
        (l1, (47.0, 33.0), "Guest Bath"),
        (l1, (54.0, 33.0), "Office"),
        (l2, (36.0, 4.0), "Primary Bedroom"),
        (l2, (54.0, 0.0), "Primary Bath"),
        (l2, (54.0, 8.0), "Closet"),
        (l2, (28.0, 15.0), "Gallery"),
        (l2, (60.0, 23.0), "Bedroom 2"),
        (l2, (33.0, 29.5), "Bedroom 3"),
        (l2, (43.5, 29.5), "Bath 2"),
        (l2, (50.5, 29.5), "Laundry"),
    ] {
        b.room(level, at, name)?;
    }
    let gwb = b.named(Category::CeilingType, "GWB")?;
    for (level, at) in [
        (l1, (34.0, 9.0)),
        (l1, (49.0, 9.0)),
        (l1, (60.0, 12.0)),
        (l1, (62.0, 22.0)),
        (l1, (27.0, 20.0)),
        (l1, (27.0, 29.0)),
        (l1, (37.0, 33.0)),
        (l1, (47.0, 33.0)),
        (l1, (54.0, 33.0)),
        (l2, (36.0, 4.0)),
        (l2, (54.0, 0.0)),
        (l2, (54.0, 8.0)),
        (l2, (60.0, 23.0)),
        (l2, (33.0, 29.5)),
        (l2, (43.5, 29.5)),
        (l2, (50.5, 29.5)),
    ] {
        studio_regen::derived::create_ceiling_in_room(b.doc, gwb, level, ft(at.0, at.1))?;
    }

    // ---------------------------------------------------------------- kitchen casework
    // Counters 3' high: the sink run under the window (a gap for the dishwasher), the
    // east run to the wall ovens and refrigerator, and an island with the cooktop.
    b.casework(
        "Kitchen Counters",
        l1,
        &[
            (54.5, 0.33, 56.0, 2.33, 0.0, 3.0),
            (58.0, 0.33, 65.67, 2.33, 0.0, 3.0),
            (63.67, 2.33, 65.67, 9.0, 0.0, 3.0),
        ],
        &[(59.5, 0.7, 62.0, 2.0, 2.25, 3.5)],
    )?;
    b.casework(
        "Kitchen Island",
        l1,
        &[(55.0, 6.5, 60.5, 10.0, 0.0, 3.0)],
        &[],
    )?;

    // ---------------------------------------------------------------- furniture and equipment
    let ffe: &[(&str, ElementId, (f64, f64), f64)] = &[
        // Living.
        ("Area Rug 9x12", l1, (34.0, 10.0), SOUTH),
        ("Sofa 96\"", l1, (34.0, 6.5), NORTH),
        ("Coffee Table", l1, (34.0, 10.0), SOUTH),
        ("Club Chair", l1, (27.6, 11.5), EAST),
        ("Club Chair", l1, (40.4, 11.5), WEST),
        ("Media Console", l1, (34.0, 17.05), SOUTH),
        ("Tall Bookcase", l1, (24.85, 14.5), EAST),
        ("End Table", l1, (28.6, 6.5), NORTH),
        // Dining.
        ("Dining Table 8-Seat", l1, (49.0, 9.0), EAST),
        ("Dining Chair", l1, (46.4, 6.0), EAST),
        ("Dining Chair", l1, (46.4, 8.0), EAST),
        ("Dining Chair", l1, (46.4, 10.0), EAST),
        ("Dining Chair", l1, (46.4, 12.0), EAST),
        ("Dining Chair", l1, (51.6, 6.0), WEST),
        ("Dining Chair", l1, (51.6, 8.0), WEST),
        ("Dining Chair", l1, (51.6, 10.0), WEST),
        ("Dining Chair", l1, (51.6, 12.0), WEST),
        // Kitchen.
        ("Dishwasher 24\"", l1, (57.0, 1.33), NORTH),
        ("Double Wall Oven", l1, (64.67, 10.25), WEST),
        ("Refrigerator French Door 36\"", l1, (64.42, 13.3), WEST),
        ("Induction Cooktop 30\"", l1, (57.75, 8.0), NORTH),
        ("Backless Counter Stool", l1, (56.2, 10.9), SOUTH),
        ("Backless Counter Stool", l1, (57.75, 10.9), SOUTH),
        ("Backless Counter Stool", l1, (59.3, 10.9), SOUTH),
        ("Espresso Machine", l1, (64.67, 5.0), WEST),
        // Foyer and mudroom.
        ("Storage Bench", l1, (64.9, 20.5), WEST),
        ("Storage Bench", l1, (27.0, 25.05), SOUTH),
        ("Shoe Cabinet", l1, (24.85, 20.0), EAST),
        // Garage.
        ("Water Heater 50 gal", l1, (22.5, 30.5), WEST),
        ("Chest Freezer", l1, (17.5, 31.0), SOUTH),
        // Guest bedroom.
        ("Queen Bed", l1, (37.0, 36.17), SOUTH),
        ("Nightstand", l1, (33.33, 38.9), SOUTH),
        ("Nightstand", l1, (40.67, 38.9), SOUTH),
        ("Dresser", l1, (41.2, 27.05), NORTH),
        // Office.
        ("Desk", l1, (54.0, 35.5), NORTH),
        ("Office Chair", l1, (54.0, 37.6), SOUTH),
        ("Tall Bookcase", l1, (50.8, 31.0), EAST),
        ("Guest Chair", l1, (55.5, 30.5), NORTH),
        // Terrace.
        ("Outdoor Sofa", l1, (34.0, -11.0), NORTH),
        ("Fire Table", l1, (34.0, -7.5), NORTH),
        ("Outdoor Lounge Chair", l1, (29.0, -6.5), EAST),
        ("Outdoor Lounge Chair", l1, (39.0, -6.5), WEST),
        ("Outdoor Dining Table", l1, (57.0, -9.0), NORTH),
        ("Outdoor Dining Chair", l1, (55.0, -6.6), SOUTH),
        ("Outdoor Dining Chair", l1, (57.0, -6.6), SOUTH),
        ("Outdoor Dining Chair", l1, (59.0, -6.6), SOUTH),
        ("Outdoor Dining Chair", l1, (55.0, -11.4), NORTH),
        ("Outdoor Dining Chair", l1, (57.0, -11.4), NORTH),
        ("Outdoor Dining Chair", l1, (59.0, -11.4), NORTH),
        ("Patio Umbrella 9'", l1, (57.0, -9.0), NORTH),
        ("Built-in Grill", l1, (64.5, -14.5), NORTH),
        ("Condensing Unit", l1, (68.5, 4.0), EAST),
        // Primary suite.
        ("King Bed", l2, (36.0, 8.3), SOUTH),
        ("Nightstand", l2, (31.67, 11.05), SOUTH),
        ("Nightstand", l2, (40.33, 11.05), SOUTH),
        ("Bed Bench", l2, (36.0, 3.9), SOUTH),
        ("Area Rug 8x10", l2, (36.0, 6.0), SOUTH),
        ("Guestroom Lounge Chair", l2, (29.5, -1.3), EAST),
        ("Dresser", l2, (44.97, 1.5), WEST),
        ("Open Closet Unit", l2, (50.0, 10.8), SOUTH),
        ("Open Closet Unit", l2, (54.0, 10.8), SOUTH),
        ("Open Closet Unit", l2, (58.0, 10.8), SOUTH),
        ("Dresser", l2, (54.0, 7.0), SOUTH),
        // Gallery.
        ("Lobby Lounge Chair", l2, (28.5, 22.5), EAST),
        ("Low Bookshelf", l2, (52.8, 15.5), WEST),
        // Bedroom 2.
        ("Queen Bed", l2, (62.17, 23.0), WEST),
        ("Nightstand", l2, (64.92, 19.33), WEST),
        ("Nightstand", l2, (64.92, 26.67), WEST),
        ("Writing Desk", l2, (55.2, 30.0), EAST),
        ("Office Chair", l2, (57.2, 30.0), WEST),
        ("Dresser", l2, (60.0, 13.05), NORTH),
        // Bedroom 3.
        ("Full Bed", l2, (29.7, 29.5), EAST),
        ("Nightstand", l2, (27.1, 26.4), EAST),
        ("Tall Chest", l2, (38.9, 31.0), WEST),
        // Laundry.
        ("Front-Load Washer", l2, (48.6, 32.35), SOUTH),
        ("Front-Load Dryer", l2, (51.1, 32.35), SOUTH),
    ];
    for (name, level, at, deg) in ffe {
        b.put(name, *level, *at, *deg)
            .with_context(|| format!("placing {name}"))?;
    }
    // Wall-mounted pieces go on the nearest wall face.
    for (name, level, at) in [
        ("TV 75\"", l1, (34.0, 17.7)),
        ("EV Charger", l1, (23.7, 14.0)),
        ("Electrical Panel", l1, (23.7, 26.5)),
        ("TV 55\"", l2, (54.3, 23.0)),
    ] {
        b.put(name, level, at, SOUTH)?;
    }

    // ---------------------------------------------------------------- lighting
    let h9 = Some(9.0 * MM_PER_FT);
    b.light(
        "4\" LED Downlight",
        l1,
        &[
            (28.0, 3.5),
            (34.0, 3.5),
            (40.0, 3.5),
            (28.0, 14.5),
            (40.0, 14.5),
            (56.0, 4.0),
            (63.0, 4.0),
            (61.5, 12.0),
            (35.0, 23.0),
            (45.0, 23.0),
            (52.0, 22.0),
            (12.0, 14.0),
        ],
        0.0,
        None,
    )?;
    b.light("8' Linear Pendant", l1, &[(49.0, 9.0)], FRAC_PI_2, None)?;
    b.light(
        "6\" Mini Pendant",
        l1,
        &[(56.3, 8.25), (57.75, 8.25), (59.2, 8.25)],
        0.0,
        None,
    )?;
    b.light("12\" Globe Pendant", l1, &[(62.0, 22.0)], 0.0, None)?;
    b.light("Floor Lamp", l1, &[(25.6, 4.0)], 0.0, None)?;
    b.light("Table Lamp", l1, &[(28.6, 6.5)], 0.0, None)?;
    b.light(
        "14\" Flush Mount",
        l1,
        &[(27.0, 21.5), (37.0, 32.0), (54.0, 32.0)],
        0.0,
        None,
    )?;
    b.light(
        "24\" Vanity Bar",
        l1,
        &[(24.4, 29.0), (47.0, 39.4)],
        0.0,
        None,
    )?;
    b.light(
        "4' LED Strip",
        l1,
        &[(8.0, 20.0), (16.0, 20.0)],
        FRAC_PI_2,
        None,
    )?;
    b.light("24\" Under-Cabinet Light", l1, &[(64.5, 0.6)], 0.0, None)?;
    b.light(
        "Exterior Wall Lantern",
        l1,
        &[(59.6, 26.5), (64.4, 26.5)],
        0.0,
        None,
    )?;
    b.light(
        "Soffit Downlight",
        l1,
        &[
            (30.0, -2.0),
            (37.0, -2.0),
            (44.0, -2.0),
            (51.0, -2.0),
            (58.0, -2.0),
        ],
        0.0,
        h9,
    )?;
    b.light(
        "42\" Bollard",
        l1,
        &[
            (59.0, 30.0),
            (59.0, 37.0),
            (59.0, 44.0),
            (65.0, 33.5),
            (65.0, 41.0),
        ],
        0.0,
        None,
    )?;
    b.light(
        "Landscape Spot",
        l1,
        &[(72.0, -8.0), (69.0, 34.0)],
        PI,
        None,
    )?;
    b.light("52\" Ceiling Fan with Light", l2, &[(36.0, 5.0)], 0.0, None)?;
    b.light(
        "4\" LED Downlight",
        l2,
        &[
            (28.5, 15.0),
            (36.0, 15.0),
            (44.0, 15.0),
            (50.5, 20.0),
            (50.0, 0.0),
            (58.0, 0.0),
        ],
        0.0,
        None,
    )?;
    b.light(
        "14\" Flush Mount",
        l2,
        &[(54.0, 8.0), (60.0, 23.0), (33.0, 29.5), (50.5, 29.5)],
        0.0,
        None,
    )?;
    b.light(
        "24\" Vanity Bar",
        l2,
        &[(54.0, 3.6), (43.5, 33.6)],
        0.0,
        None,
    )?;
    b.light(
        "Table Lamp",
        l2,
        &[(31.67, 11.05), (40.33, 11.05)],
        0.0,
        None,
    )?;
    b.light("Wall Sconce", l2, &[(26.4, 22.5)], 0.0, None)?;

    // ---------------------------------------------------------------- site and landscape
    let lawn = studio_core::planting::ground_material(b.doc, "site-lawn-lush")?;
    studio_core::planting::set_ground(b.doc, Some(lawn))?;
    b.ground(
        l1,
        "site-bluestone-pattern",
        &[(20.0, -17.0), (70.0, -17.0), (70.0, 0.0), (20.0, 0.0)],
    )?;
    b.ground(
        l1,
        "site-concrete-broom",
        &[(-30.0, 9.0), (0.0, 9.0), (0.0, 31.0), (-30.0, 31.0)],
    )?;
    b.ground(
        l1,
        "site-pavers-running",
        &[(60.0, 26.0), (64.0, 26.0), (64.0, 50.0), (60.0, 50.0)],
    )?;
    b.ground(
        l1,
        "site-pea-gravel",
        &[(30.0, 40.0), (58.0, 40.0), (58.0, 44.0), (30.0, 44.0)],
    )?;
    b.plants("Honey Locust", l1, &[(78.5, -12.5)], 0.5)?;
    b.plants("Japanese Maple, Red", l1, &[(68.0, 34.0)], 1.0)?;
    b.plants("Birch Clump", l1, &[(-16.0, 46.0), (78.0, 24.0)], 0.7)?;
    b.plants("Coast Live Oak", l1, &[(-34.0, -24.0)], 0.55)?;
    b.plants("Italian Cypress", l1, &[(-4.0, 3.0), (2.0, 3.0)], 1.0)?;
    let reeds: Vec<(f64, f64)> = (0..9).map(|i| (32.0 + 3.0 * f64::from(i), 42.0)).collect();
    b.plants("Feather Reed Grass", l1, &reeds, 1.0)?;
    b.plants(
        "Boxwood, Round",
        l1,
        &[(58.5, 47.0), (65.5, 47.0), (58.5, 29.0)],
        1.0,
    )?;
    // Rolling ground beyond the garden (ADR-095): the lot flat round the house, rising
    // into gentle lawn berms, as a landscape architect grades a modern site. The site
    // keeps the default location (the sun is unchanged) and has no address or parcel.
    {
        let center = (33.0, 18.0);
        let mounds: [(f64, f64, f64, f64); 5] = [
            (-70.0, 45.0, 7.0, 48.0),
            (-40.0, -70.0, 5.0, 42.0),
            (130.0, 115.0, 6.0, 52.0),
            (20.0, 160.0, 8.0, 62.0),
            (-120.0, -10.0, 9.0, 60.0),
        ];
        let height = |x: f64, y: f64| -> f64 {
            let d = (x - center.0).hypot(y - center.1);
            let t = ((d - 95.0) / 40.0).clamp(0.0, 1.0);
            let ease = t * t * (3.0 - 2.0 * t);
            let hills: f64 = mounds
                .iter()
                .map(|(mx, my, h, r)| h * (-((x - mx).powi(2) + (y - my).powi(2)) / (r * r)).exp())
                .sum();
            ease * (hills + 0.025 * (d - 135.0).max(0.0))
        };
        let (x0, y0, spacing, n) = (center.0 - 600.0, center.1 - 600.0, 15.0, 81u32);
        let z: Vec<f32> = (0..n)
            .flat_map(|j| {
                (0..n).map(move |i| {
                    let (x, y) = (x0 + f64::from(i) * spacing, y0 + f64::from(j) * spacing);
                    (height(x, y) * MM_PER_FT) as f32
                })
            })
            .collect();
        b.doc.transact("Grade the site", |tx| {
            Ok(tx.insert(ElementData::Site {
                address: String::new(),
                lat: 39.8,
                lon: -98.6,
                boundary: vec![],
                parcel: studio_core::site::ParcelInfo::default(),
                offset: Pt::default(),
                rotation: 0.0,
                base_elevation: 0.0,
                contour: 2.0 * MM_PER_FT,
                topo: Some(studio_core::site::Topo {
                    x0: x0 * MM_PER_FT,
                    y0: y0 * MM_PER_FT,
                    spacing: spacing * MM_PER_FT,
                    nx: n,
                    ny: n,
                    z,
                    resolution: 3.0,
                }),
            }))
        })?;
    }
    // Planting beds (ADR-095), laid out as a landscape architect would for the hero view:
    // curved beds of hardwood mulch, massed perennials and shrubs, boulders among them.
    let blob = |cx: f64, cy: f64, rx: f64, ry: f64, turn: f64| -> Vec<(f64, f64)> {
        (0..24)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 24.0;
                let k = 1.0 + 0.1 * (3.0 * a + turn).sin() + 0.05 * (5.0 * a + 1.0).cos();
                let (x, y) = (rx * k * a.cos(), ry * k * a.sin());
                let (s, co) = turn.sin_cos();
                (cx + x * co - y * s, cy + x * s + y * co)
            })
            .collect()
    };
    // Plants scattered through a bed, sunflower-spaced so they mass without a grid.
    let bed = |cx: f64, cy: f64, n: usize, r: f64| -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| {
                let a = i as f64 * 2.399;
                let d = r * ((i as f64 + 0.5) / n as f64).sqrt();
                (cx + d * a.cos(), cy + d * a.sin())
            })
            .collect()
    };
    // Left foreground: blue hydrangeas massed on mulch, salvia in front.
    b.ground(l1, "site-bark-mulch", &blob(61.0, -38.0, 7.5, 5.0, 0.4))?;
    b.plants("Bigleaf Hydrangea", l1, &bed(60.5, -38.5, 7, 4.6), 1.0)?;
    b.plants("Salvia", l1, &bed(65.0, -35.0, 5, 2.0), 1.0)?;
    b.plants("Boxwood, Round", l1, &[(55.5, -40.0)], 1.1)?;
    // Right foreground, close to the lens: salvia, lavender and Russian sage spilling out
    // of the frame.
    b.ground(l1, "site-bark-mulch", &blob(84.5, -28.0, 6.0, 4.5, -0.3))?;
    b.plants("Salvia", l1, &bed(83.5, -28.5, 10, 4.0), 1.1)?;
    b.plants("English Lavender", l1, &bed(86.5, -25.5, 5, 2.2), 1.1)?;
    b.plants("Salvia", l1, &bed(81.0, -24.5, 4, 1.6), 1.0)?;
    // The bed at the terrace's southeast corner, round the olive: a pile of fieldstone
    // and a boulder, lavender, grasses and agave, liriope along its edge.
    b.ground(l1, "site-bark-mulch", &blob(71.0, -18.5, 8.5, 4.8, 0.55))?;
    b.plants(
        "Stacked Ledge Stones",
        l1,
        &[(69.5, -21.0), (72.0, -22.5)],
        1.0,
    )?;
    b.plants("Boulder Cluster, Fieldstone", l1, &[(66.5, -22.5)], 0.6)?;
    b.plants("English Lavender", l1, &bed(74.0, -18.5, 6, 2.2), 1.0)?;
    b.plants("Fountain Grass", l1, &bed(66.5, -18.5, 4, 2.0), 0.8)?;
    b.plants("Blue Agave", l1, &[(72.5, -15.5), (77.5, -14.5)], 0.7)?;
    b.plants("Liriope", l1, &bed(68.0, -23.5, 6, 2.5), 1.0)?;
    // The terrace's west end and the east wall: agave, lavender and boxwood.
    b.ground(l1, "site-bark-mulch", &blob(23.0, -18.5, 4.5, 2.8, 0.0))?;
    b.plants("English Lavender", l1, &bed(23.5, -18.5, 5, 2.4), 1.0)?;
    b.plants("Blue Agave", l1, &[(19.5, -17.0), (71.0, -2.0)], 0.8)?;
    b.plants("Boxwood, Round", l1, &[(26.5, -19.5), (21.0, -20.0)], 0.9)?;
    b.plants("Fountain Grass", l1, &[(70.0, 1.5), (72.0, 4.0)], 1.0)?;
    // Bluestone stepping stones through the lawn, square to the path.
    let (dx, dy) = (-2.4_f64, 3.0_f64);
    let l = dx.hypot(dy);
    let (ux, uy) = (dx / l, dy / l);
    let (vx, vy) = (-uy, ux);
    for i in 0..8 {
        let (x, y) = (
            79.0 + dx * 1.15 * f64::from(i) - 1.0,
            -40.0 + dy * 1.15 * f64::from(i),
        );
        let (a, w) = (0.75, 1.3);
        b.ground(
            l1,
            "site-bluestone-slab",
            &[
                (x - ux * a - vx * w, y - uy * a - vy * w),
                (x + ux * a - vx * w, y + uy * a - vy * w),
                (x + ux * a + vx * w, y + uy * a + vy * w),
                (x - ux * a + vx * w, y - uy * a + vy * w),
            ],
        )?;
    }
    // A wooded edge around the lot, so the horizon is trees, not a bare plain.
    let ring = |r: f64, from: f64, to: f64, n: usize, jitter: f64| -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| {
                let t = from + (to - from) * i as f64 / (n - 1).max(1) as f64;
                let a = t.to_radians();
                let k = 1.0 + jitter * ((i as f64 * 1.37).sin());
                (33.0 + r * k * a.cos(), 18.0 + r * k * a.sin())
            })
            .collect()
    };
    b.plants("Red Maple", l1, &ring(150.0, 20.0, 160.0, 9, 0.12), 1.1)?;
    b.plants("White Oak", l1, &ring(175.0, 35.0, 150.0, 7, 0.1), 1.2)?;
    b.plants("Tulip Tree", l1, &ring(135.0, 160.0, 230.0, 5, 0.1), 1.1)?;
    b.plants("Norway Spruce", l1, &ring(165.0, 60.0, 125.0, 6, 0.15), 1.0)?;
    b.plants("River Birch", l1, &ring(120.0, -10.0, 40.0, 4, 0.1), 1.0)?;
    b.plants("Sweetgum", l1, &ring(190.0, 0.0, 70.0, 5, 0.12), 1.1)?;
    b.plants("Pin Oak", l1, &ring(125.0, 185.0, 255.0, 6, 0.15), 1.1)?;
    b.plants(
        "Littleleaf Linden",
        l1,
        &ring(160.0, 175.0, 265.0, 6, 0.12),
        1.0,
    )?;
    b.plants(
        "Eastern White Pine",
        l1,
        &ring(200.0, 150.0, 250.0, 5, 0.1),
        1.0,
    )?;
    // ---------------------------------------------------------------- materials
    let paint = |doc: &mut Document, ids: &[ElementId], preset: &str| -> anyhow::Result<()> {
        let m = studio_core::library::add_preset(doc, preset)?;
        studio_core::library::apply_to(doc, ids, m)?;
        Ok(())
    };
    paint(b.doc, &base[..1], "plaster-stucco-white")?;
    paint(b.doc, &upper[..1], "siding-cedar-lap-stained")?;
    // Outdoor furniture in the finishes of a resort terrace (ADR-095): grey resin wicker
    // under light cushions, teak, a stainless grill on a dark base, a canvas umbrella.
    for (name, main, accent, cushion) in [
        (
            "Outdoor Sofa",
            [86, 84, 80],
            [60, 58, 56],
            Some([214, 212, 206]),
        ),
        (
            "Outdoor Lounge Chair",
            [86, 84, 80],
            [60, 58, 56],
            Some([214, 212, 206]),
        ),
        ("Outdoor Dining Table", [152, 106, 70], [140, 98, 64], None),
        ("Outdoor Dining Chair", [152, 106, 70], [140, 98, 64], None),
        ("Patio Umbrella 9'", [236, 232, 222], [150, 106, 70], None),
        ("Built-in Grill", [196, 198, 200], [58, 54, 50], None),
        ("Fire Table", [148, 146, 140], [70, 68, 64], None),
    ] {
        let ids: Vec<ElementId> = b
            .doc
            .of(Category::FfeType)
            .filter(|e| e.data.name() == name)
            .map(|e| e.id)
            .collect();
        b.doc.transact("Finish outdoor furniture", |tx| {
            for id in &ids {
                tx.modify(*id, |d| {
                    if let ElementData::FfeType { spec, .. } = d {
                        spec.color = main;
                        spec.accent = accent;
                        spec.cushion = cushion;
                    }
                })?;
            }
            Ok(())
        })?;
    }
    let floors: Vec<ElementId> = b.doc.of(Category::Floor).map(|e| e.id).collect();
    let (ground_floor, upper_floor): (Vec<ElementId>, Vec<ElementId>) = floors
        .into_iter()
        .partition(|id| b.doc.data(*id).ok().and_then(|d| d.level()) == Some(l1));
    paint(b.doc, &ground_floor, "concrete-polished")?;
    paint(b.doc, &upper_floor, "wood-white-oak-floor")?;
    let roofs: Vec<ElementId> = b.doc.of(Category::Roof).map(|e| e.id).collect();
    paint(b.doc, &roofs[..1], "roof-tpo-white")?;
    let columns: Vec<ElementId> = b.doc.of(Category::Column).map(|e| e.id).collect();
    paint(b.doc, &columns[..1], "metal-matte-black")?;

    // Interior elevations of the kitchen (ADR-021).
    let marker = studio_regen::derived::create_elevation_marker(b.doc, l1, ft(58.5, 12.5), true)?;
    for dir in ["North", "East", "South", "West"] {
        studio_regen::derived::set_property(b.doc, marker, &format!("view_{dir}"), "yes")?;
    }
    // The hero view: across the lawn from the southeast, at eye height, level (two-point
    // perspective), the cantilever and the folding glass wall in late light.
    let hero = studio_core::camera::create_camera(
        b.doc,
        l1,
        ft(84.0, -40.0),
        ft(40.0, 6.0),
        5.0 * MM_PER_FT,
    )?;
    ops::set_property(b.doc, hero, "name", "Hero View - Southeast", 0)?;
    documents(&mut b, l1, l2)
}

/// The drawing set: plans, elevations, a section and schedules on ARCH D sheets.
fn documents(b: &mut B<'_>, l1: ElementId, l2: ElementId) -> anyhow::Result<()> {
    use studio_core::{ScheduleKind, SheetSize};
    let plan1 = b.view_where(&|k, _| matches!(k, ViewKind::FloorPlan { level } if *level == l1))?;
    let plan2 = b.view_where(&|k, _| matches!(k, ViewKind::FloorPlan { level } if *level == l2))?;
    // Overall dimensions on the Level 1 plan, outside face to outside face.
    let face = 3.75 / 12.0;
    let six = 6.0 * MM_PER_FT;
    ops::create_dimension(
        b.doc,
        plan1,
        ft(-face, -face),
        ft(66.0 + face, -face),
        -six - 4.0 * MM_PER_FT,
    )?;
    ops::create_dimension(b.doc, plan1, ft(-face, 40.0 + face), ft(-face, -face), -six)?;
    ops::create_dimension(
        b.doc,
        plan1,
        ft(0.0, 40.0 + face),
        ft(24.0, 40.0 + face),
        3.0 * MM_PER_FT,
    )?;
    ops::create_dimension(
        b.doc,
        plan1,
        ft(24.0, 40.0 + face),
        ft(44.0, 40.0 + face),
        3.0 * MM_PER_FT,
    )?;
    ops::create_dimension(
        b.doc,
        plan1,
        ft(44.0, 40.0 + face),
        ft(66.0, 40.0 + face),
        3.0 * MM_PER_FT,
    )?;
    let section = ops::create_section(b.doc, ft(49.0, -22.0), ft(49.0, 48.0))?;
    let cross = ops::create_section(b.doc, ft(-6.0, 22.0), ft(72.0, 22.0))?;
    let schedule = |b: &B<'_>, kind: ScheduleKind| {
        b.view_where(&|k, _| matches!(k, ViewKind::Schedule { kind: s } if *s == kind))
    };
    let elevation = |b: &B<'_>, name: &str| {
        b.view_where(&|k, n| matches!(k, ViewKind::Elevation { .. }) && n == name)
    };
    let p = Pt::new;
    let cover = ops::create_sheet(b.doc, "Cover Sheet", SheetSize::ArchD)?;
    ops::set_property(b.doc, cover, "number", "A0.0", 0)?;
    // The hero rendering (ADR-095), path traced from the Hero View camera, across the top.
    {
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(HERO_RENDERING);
        let hero = studio_core::renderings::save(
            b.doc,
            "Hero View - Southeast - Rendering",
            "image/jpeg",
            data,
            1600,
            900,
        )?;
        ops::place_view(b.doc, cover, hero, p(400.0, 410.0))?;
    }
    ops::place_view(
        b.doc,
        cover,
        schedule(b, ScheduleKind::Sheets)?,
        p(170.0, 150.0),
    )?;
    ops::place_view(
        b.doc,
        cover,
        schedule(b, ScheduleKind::Rooms)?,
        p(420.0, 150.0),
    )?;
    ops::place_view(
        b.doc,
        cover,
        schedule(b, ScheduleKind::Doors)?,
        p(670.0, 150.0),
    )?;
    let plans = ops::create_sheet(b.doc, "Floor Plans", SheetSize::ArchD)?;
    ops::set_property(b.doc, plans, "number", "A1.0", 0)?;
    ops::place_view(b.doc, plans, plan1, p(225.0, 320.0))?;
    ops::place_view(b.doc, plans, plan2, p(600.0, 320.0))?;
    let elevations = ops::create_sheet(b.doc, "Exterior Elevations", SheetSize::ArchD)?;
    ops::set_property(b.doc, elevations, "number", "A2.0", 0)?;
    for (name, at) in [
        ("South", p(225.0, 440.0)),
        ("North", p(600.0, 440.0)),
        ("East", p(225.0, 200.0)),
        ("West", p(600.0, 200.0)),
    ] {
        ops::place_view(b.doc, elevations, elevation(b, name)?, at)?;
    }
    let sections = ops::create_sheet(b.doc, "Building Sections", SheetSize::ArchD)?;
    ops::set_property(b.doc, sections, "number", "A3.0", 0)?;
    ops::place_view(b.doc, sections, section, p(225.0, 320.0))?;
    ops::place_view(b.doc, sections, cross, p(600.0, 320.0))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn house() -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        build_modern(&mut doc).unwrap();
        doc
    }

    #[test]
    fn the_modern_house_is_built_from_everything() {
        let doc = house();
        let count = |c| doc.of(c).count();
        assert!(
            count(Category::Wall) >= 30,
            "{} walls",
            count(Category::Wall)
        );
        assert!(count(Category::Door) >= 18);
        assert!(count(Category::Window) >= 24);
        assert_eq!(count(Category::Room), 19);
        assert_eq!(count(Category::Stair), 1);
        assert_eq!(count(Category::Roof), 4);
        assert_eq!(count(Category::Column), 2);
        assert!(count(Category::Furniture) >= 60);
        assert!(count(Category::SpecialtyEquipment) >= 12);
        assert!(count(Category::LightingFixture) >= 50);
        assert!(count(Category::Planting) >= 35);
        assert_eq!(count(Category::Casework), 2);
        assert_eq!(count(Category::Sheet), 4);
        // Every room closes (an open boundary would leave it unplaced, with no area).
        let model = studio_regen::regenerate(&doc);
        for e in doc.of(Category::Room) {
            let area = model
                .rooms
                .iter()
                .find(|r| r.id == e.id)
                .map_or(0.0, |r| r.area());
            assert!(
                area > 20.0 * MM_PER_FT * MM_PER_FT,
                "{} has no area",
                e.data.name()
            );
        }
        // Nothing overlaps another opening in its wall.
        let get = |id: ElementId| doc.get(id).map(|e| &e.data);
        let all = doc
            .of(Category::Door)
            .chain(doc.of(Category::Window))
            .map(|e| (e.id, &e.data));
        studio_core::hosting::validate_openings(all, &get).unwrap();
        // It draws in 3D (furniture, fixtures and plants included) and prints as a set.
        let meshes = studio_views::meshes(&doc);
        for cat in [
            Category::Furniture,
            Category::LightingFixture,
            Category::Roof,
            Category::Stair,
        ] {
            assert!(meshes.iter().any(|m| m.category == cat), "no {cat:?} in 3D");
        }
        let sheets: Vec<ElementId> = doc.of(Category::Sheet).map(|e| e.id).collect();
        let pdf = studio_sheets::pdf::export_pdf(&doc, &sheets, "2026-09-30").unwrap();
        assert!(pdf.len() > 10_000);
    }
}

#[cfg(test)]
mod preview {
    use super::*;
    use studio_views::{FillKind, Prim};

    /// The hero rendering is on the Cover Sheet (ADR-095), and the PDF embeds it.
    #[test]
    fn the_cover_sheet_carries_the_hero_rendering() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        build_modern(&mut doc).unwrap();
        let (cover, _, _) = ops::sheets(&doc)
            .into_iter()
            .find(|(_, number, _)| number == "A0.0")
            .unwrap();
        let on_cover: Vec<ElementId> = doc
            .of(Category::Viewport)
            .filter_map(|e| match &e.data {
                ElementData::Viewport { sheet, view, .. } if *sheet == cover => Some(*view),
                _ => None,
            })
            .collect();
        let hero = on_cover
            .iter()
            .find(|v| studio_core::renderings::image_of(&doc, **v).is_some())
            .expect("a rendering on the cover");
        let (mime, _, w, h, _) = studio_core::renderings::image_of(&doc, *hero).unwrap();
        assert_eq!((mime, w, h), ("image/jpeg", 1600, 900));
        let pdf = studio_sheets::export_pdf(&doc, &[cover], "2026-10-01").unwrap();
        // The JPEG goes in whole (a DCT-encoded image XObject).
        assert!(pdf.len() > HERO_RENDERING.len());
        if let Ok(out) = std::env::var("COVER_PDF") {
            std::fs::write(out, pdf).unwrap();
        }
    }

    /// Dev aid: `SAMPLE_SVG=dir cargo test -p rufplan-studio write_modern_svgs -- --ignored`
    /// draws the Modern House's views as SVG.
    #[test]
    #[ignore]
    fn write_modern_svgs() {
        let dir = std::env::var("SAMPLE_SVG").unwrap();
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        build_modern(&mut doc).unwrap();
        let views: Vec<(ElementId, String)> = doc
            .of(Category::View)
            .filter(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. }
                            | ViewKind::Elevation { .. }
                            | ViewKind::Section { .. },
                        ..
                    }
                )
            })
            .map(|e| (e.id, e.data.name()))
            .collect();
        for (v, name) in views {
            let Some(dl) = studio_views::display_list(&doc, v) else {
                continue;
            };
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            let mut grow = |p: &[f64; 2]| {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            };
            for i in &dl.items {
                match &i.prim {
                    Prim::Line { pts, .. } => pts.iter().for_each(&mut grow),
                    Prim::Fill { rings, .. } => rings.iter().flatten().for_each(&mut grow),
                    _ => {}
                }
            }
            let s = 1600.0 / (x1 - x0).max(y1 - y0);
            let tx = |p: &[f64; 2]| {
                format!(
                    "{:.1},{:.1}",
                    (p[0] - x0) * s + 20.0,
                    (y1 - p[1]) * s + 20.0
                )
            };
            let mut svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{:.0}' height='{:.0}' style='background:#fff'>",
                (x1 - x0) * s + 40.0,
                (y1 - y0) * s + 40.0
            );
            for i in &dl.items {
                match &i.prim {
                    Prim::Image { .. } => {}
                    Prim::Fill { rings, fill } => {
                        let color = match fill {
                            FillKind::Paper => "#fff",
                            _ => "#bbb",
                        };
                        let d: String = rings
                            .iter()
                            .map(|r| {
                                format!("M{}Z", r.iter().map(tx).collect::<Vec<_>>().join("L"))
                            })
                            .collect();
                        svg += &format!("<path d='{d}' fill='{color}' fill-rule='evenodd'/>");
                    }
                    Prim::Line { pts, closed, w, .. } => {
                        let d = pts.iter().map(tx).collect::<Vec<_>>().join(" ");
                        let tag = if *closed { "polygon" } else { "polyline" };
                        svg += &format!(
                            "<{tag} points='{d}' fill='none' stroke='#000' stroke-width='{:.1}'/>",
                            f64::from(*w) * 0.5
                        );
                    }
                    Prim::Text { at, text, size, .. } => {
                        let t = text.replace('&', "&amp;").replace('<', "&lt;");
                        svg += &format!(
                            "<text x='{:.1}' y='{:.1}' font-size='{:.1}' font-family='Arial'>{t}</text>",
                            (at[0] - x0) * s + 20.0,
                            (y1 - at[1]) * s + 20.0,
                            (size * s).max(6.0)
                        );
                    }
                    Prim::Circle { c, r, .. } => {
                        svg += &format!(
                            "<circle cx='{:.1}' cy='{:.1}' r='{:.1}' fill='none' stroke='#000' stroke-width='0.5'/>",
                            (c[0] - x0) * s + 20.0,
                            (y1 - c[1]) * s + 20.0,
                            r * s
                        );
                    }
                }
            }
            svg += "</svg>";
            std::fs::write(format!("{dir}/{}.svg", name.replace(['/', ' '], "_")), svg).unwrap();
        }
    }
}
