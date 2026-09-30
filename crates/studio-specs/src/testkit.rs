//! A small house for the tests: four exterior walls, a door, a window and three rooms.

use studio_core::units::MM_PER_FT;
use studio_core::{ops, Category, Document};
use studio_geom::Pt;

pub fn house() -> Document {
    let mut doc = Document::new();
    ops::seed_default_project(&mut doc).unwrap();
    let l1 = doc.levels()[0].0;
    let wt = doc
        .of(Category::WallType)
        .find(|e| e.data.name().starts_with("Exterior"))
        .map(|e| e.id)
        .unwrap();
    let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
    let c = [
        Pt::new(0.0, 0.0),
        Pt::new(w, 0.0),
        Pt::new(w, h),
        Pt::new(0.0, h),
    ];
    let walls: Vec<_> = (0..4)
        .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
        .collect();
    let dt = ops::first_of(&doc, Category::DoorType).unwrap();
    ops::create_door(&mut doc, dt, walls[0], 10.0 * MM_PER_FT, false).unwrap();
    let wn = ops::first_of(&doc, Category::WindowType).unwrap();
    ops::create_window(&mut doc, wn, walls[1], 12.0 * MM_PER_FT, false).unwrap();
    for (name, x) in [("Bedroom", 5.0), ("Kitchen", 20.0), ("Bath", 35.0)] {
        let r = ops::create_room(&mut doc, l1, Pt::new(x * MM_PER_FT, 15.0 * MM_PER_FT)).unwrap();
        ops::set_property(&mut doc, r, "name", name, 0).unwrap();
    }
    doc
}
