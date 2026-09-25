//! Math, 2D polygon operations and prismatic solids.
//!
//! All lengths are millimetres. The native kernel only handles vertical extrusions of
//! simple polygons (ADR-006), which covers walls, floors and ceilings.

pub mod tol {
    /// Linear tolerance in mm.
    pub const LINEAR: f64 = 0.01;
    /// Angular tolerance in radians.
    pub const ANGULAR: f64 = 1e-9;
    /// Distance under which two points are treated as the same join point, in mm.
    pub const JOIN: f64 = 1.0;
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A 2D point or vector in mm.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

// Named methods rather than operator traits keep chained vector math explicit.
#[allow(clippy::should_implement_trait)]
impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn add(self, o: Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
    pub fn sub(self, o: Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
    pub fn scale(self, k: f64) -> Pt {
        Pt::new(self.x * k, self.y * k)
    }
    pub fn dot(self, o: Pt) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: Pt) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn dist(self, o: Pt) -> f64 {
        self.sub(o).len()
    }
    /// Unit vector, or zero for a zero-length vector.
    pub fn norm(self) -> Pt {
        let l = self.len();
        if l < tol::LINEAR {
            Pt::default()
        } else {
            self.scale(1.0 / l)
        }
    }
    /// Rotated 90° counter-clockwise.
    pub fn perp(self) -> Pt {
        Pt::new(-self.y, self.x)
    }
    pub fn lerp(self, o: Pt, t: f64) -> Pt {
        self.add(o.sub(self).scale(t))
    }
}

/// Intersection of the infinite lines `p + t*d` and `q + s*e`, or None if parallel.
pub fn line_intersection(p: Pt, d: Pt, q: Pt, e: Pt) -> Option<Pt> {
    let denom = d.cross(e);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = q.sub(p).cross(e) / denom;
    Some(p.add(d.scale(t)))
}

/// Parameter `t` in [0, 1] of the closest point on segment ab to p, and that distance.
pub fn project_to_segment(p: Pt, a: Pt, b: Pt) -> (f64, f64) {
    let ab = b.sub(a);
    let l2 = ab.dot(ab);
    if l2 < tol::LINEAR * tol::LINEAR {
        return (0.0, p.dist(a));
    }
    let t = (p.sub(a).dot(ab) / l2).clamp(0.0, 1.0);
    (t, p.dist(a.lerp(b, t)))
}

/// Signed area of a ring in mm² (positive = counter-clockwise).
pub fn signed_area(ring: &[Pt]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| ring[i].cross(ring[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

/// Ray-casting point-in-ring test.
pub fn point_in_ring(p: Pt, ring: &[Pt]) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A polygon with an outer ring and optional holes. Rings are not closed (no repeated
/// first point).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Poly {
    pub outer: Vec<Pt>,
    pub holes: Vec<Vec<Pt>>,
}

impl Poly {
    pub fn simple(outer: Vec<Pt>) -> Self {
        Self {
            outer,
            holes: vec![],
        }
    }
    pub fn contains(&self, p: Pt) -> bool {
        point_in_ring(p, &self.outer) && !self.holes.iter().any(|h| point_in_ring(p, h))
    }
    pub fn area(&self) -> f64 {
        signed_area(&self.outer).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }
}

fn to_geo(p: &Poly) -> geo::Polygon<f64> {
    let ring = |r: &[Pt]| geo::LineString::from(r.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>());
    geo::Polygon::new(ring(&p.outer), p.holes.iter().map(|h| ring(h)).collect())
}

fn from_geo_ring(ls: &geo::LineString<f64>) -> Vec<Pt> {
    let mut pts: Vec<Pt> = ls.coords().map(|c| Pt::new(c.x, c.y)).collect();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    simplify_ring(pts)
}

/// Drops vertices that lie within 0.05 mm of the line through their neighbours.
pub fn simplify_ring(mut pts: Vec<Pt>) -> Vec<Pt> {
    let mut changed = true;
    while changed && pts.len() > 3 {
        changed = false;
        let n = pts.len();
        for i in 0..n {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (_, d) = project_to_segment(b, a, c);
            if d < 0.05 || a.dist(b) < 0.05 {
                pts.remove(i);
                changed = true;
                break;
            }
        }
    }
    pts
}

/// Offsets a ring outward by `d` mm by moving each edge along its normal and
/// re-intersecting neighbours. Exact for convex rings; fine for small `d` otherwise.
pub fn offset_ring(ring: &[Pt], d: f64) -> Vec<Pt> {
    let n = ring.len();
    if n < 3 {
        return ring.to_vec();
    }
    // Outward normal of a CCW ring is the clockwise perpendicular of each edge.
    let sign = if signed_area(ring) > 0.0 { -1.0 } else { 1.0 };
    let edge = |i: usize| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let dir = b.sub(a).norm();
        (a.add(dir.perp().scale(sign * d)), dir)
    };
    (0..n)
        .map(|i| {
            let (p0, d0) = edge((i + n - 1) % n);
            let (p1, d1) = edge(i);
            line_intersection(p0, d0, p1, d1).unwrap_or(p1)
        })
        .collect()
}

/// Union of polygons. Touching or overlapping inputs merge into single outlines.
///
/// Inputs are grown by `tol::LINEAR` first: the boolean engine can leave polygons that
/// share an edge exactly (e.g. mitered wall corners) as separate pieces, while
/// overlapping ones always merge. The result is at most 0.01 mm larger than exact.
///
/// Inputs whose bounding boxes touch are grouped into clusters (separate buildings or
/// wings merge independently), and each cluster is merged pairwise in a balanced tree, so
/// the cost grows about as n·log n rather than n² for n walls.
pub fn union_all(polys: &[Poly]) -> Vec<Poly> {
    use geo::BooleanOps;
    let items: Vec<(Pt, Pt, geo::MultiPolygon<f64>)> = polys
        .iter()
        .filter(|p| p.outer.len() >= 3 && signed_area(&p.outer).abs() >= 1.0)
        .map(|p| {
            let grown = Poly {
                outer: offset_ring(&p.outer, tol::LINEAR),
                holes: p.holes.clone(),
            };
            let (mut lo, mut hi) = (grown.outer[0], grown.outer[0]);
            for q in &grown.outer {
                lo = Pt::new(lo.x.min(q.x), lo.y.min(q.y));
                hi = Pt::new(hi.x.max(q.x), hi.y.max(q.y));
            }
            (lo, hi, geo::MultiPolygon::new(vec![to_geo(&grown)]))
        })
        .collect();
    // Union-find over touching bounding boxes.
    let n = items.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let slop = 0.1;
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (&items[i], &items[j]);
            if a.0.x <= b.1.x + slop
                && b.0.x <= a.1.x + slop
                && a.0.y <= b.1.y + slop
                && b.0.y <= a.1.y + slop
            {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                if ri != rj {
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
        }
    }
    let mut clusters: Vec<Vec<geo::MultiPolygon<f64>>> = vec![];
    let mut slot: Vec<Option<usize>> = vec![None; n];
    for (i, item) in items.into_iter().enumerate() {
        let r = root(&mut parent, i);
        let k = *slot[r].get_or_insert_with(|| {
            clusters.push(vec![]);
            clusters.len() - 1
        });
        clusters[k].push(item.2);
    }
    let mut out = vec![];
    for mut level in clusters {
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            let mut it = level.into_iter();
            while let Some(a) = it.next() {
                next.push(match it.next() {
                    Some(b) => a.union(&b),
                    None => a,
                });
            }
            level = next;
        }
        if let Some(m) = level.pop() {
            out.extend(m.0.iter().map(|g| Poly {
                outer: from_geo_ring(g.exterior()),
                holes: g.interiors().iter().map(from_geo_ring).collect(),
            }));
        }
    }
    out
}

/// Clips a convex or simple ring to the half-plane `(q - p) · n >= 0` (Sutherland–Hodgman).
pub fn clip_half_plane(ring: &[Pt], p: Pt, n: Pt) -> Vec<Pt> {
    let side = |q: Pt| q.sub(p).dot(n);
    let mut out = vec![];
    let len = ring.len();
    for i in 0..len {
        let (a, b) = (ring[i], ring[(i + 1) % len]);
        let (sa, sb) = (side(a), side(b));
        if sa >= 0.0 {
            out.push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            out.push(a.lerp(b, sa / (sa - sb)));
        }
    }
    out
}

/// Triangulates a polygon (with holes). Returns vertex list and triangle index triples.
pub fn triangulate(p: &Poly) -> (Vec<Pt>, Vec<[usize; 3]>) {
    let mut verts: Vec<Pt> = p.outer.clone();
    let mut hole_starts = vec![];
    for h in &p.holes {
        hole_starts.push(verts.len());
        verts.extend_from_slice(h);
    }
    let flat: Vec<f64> = verts.iter().flat_map(|v| [v.x, v.y]).collect();
    let idx = earcutr::earcut(&flat, &hole_starts, 2).unwrap_or_default();
    let tris = idx.as_chunks::<3>().0.to_vec();
    (verts, tris)
}

/// A vertical extrusion of `base` from `z0` to `z1` (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct Prism {
    pub base: Poly,
    pub z0: f64,
    pub z1: f64,
}

impl Prism {
    /// Flat-shaded triangle soup: 9 floats per triangle (x, y, z per vertex), z-up.
    pub fn triangles(&self) -> Vec<f32> {
        let mut out = vec![];
        let (verts, tris) = triangulate(&self.base);
        let ccw = signed_area(&self.base.outer) > 0.0;
        let mut push = |a: Pt, az: f64, b: Pt, bz: f64, c: Pt, cz: f64| {
            for (p, z) in [(a, az), (b, bz), (c, cz)] {
                out.extend_from_slice(&[p.x as f32, p.y as f32, z as f32]);
            }
        };
        for t in &tris {
            let (a, b, c) = (verts[t[0]], verts[t[1]], verts[t[2]]);
            // Orient caps outward: top faces +z, bottom faces -z.
            let up = b.sub(a).cross(c.sub(a)) > 0.0;
            if up {
                push(a, self.z1, b, self.z1, c, self.z1);
                push(a, self.z0, c, self.z0, b, self.z0);
            } else {
                push(a, self.z1, c, self.z1, b, self.z1);
                push(a, self.z0, b, self.z0, c, self.z0);
            }
        }
        let mut side = |ring: &[Pt], outward_ccw: bool| {
            let n = ring.len();
            for i in 0..n {
                let (mut a, mut b) = (ring[i], ring[(i + 1) % n]);
                if !outward_ccw {
                    std::mem::swap(&mut a, &mut b);
                }
                push(a, self.z0, b, self.z0, b, self.z1);
                push(a, self.z0, b, self.z1, a, self.z1);
            }
        };
        side(&self.base.outer, ccw);
        for h in &self.base.holes {
            side(h, signed_area(h) < 0.0);
        }
        out
    }
}

/// True when a ring turns the same way at every corner (collinear corners allowed).
pub fn is_convex(ring: &[Pt]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut sign = 0.0;
    for i in 0..n {
        let (a, b, c) = (ring[i], ring[(i + 1) % n], ring[(i + 2) % n]);
        let z = b.sub(a).cross(c.sub(b));
        if z.abs() < 1e-6 * a.dist(b).max(1.0) * b.dist(c).max(1.0) {
            continue;
        }
        if sign == 0.0 {
            sign = z.signum();
        } else if z.signum() != sign {
            return false;
        }
    }
    true
}

fn rotate(p: Pt, cos: f64, sin: f64) -> Pt {
    Pt::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos)
}

/// Covers a right-angled (rectilinear) polygon with maximal rectangles that may overlap —
/// the "main block and wings" of an L-, T- or U-shaped plan. The polygon may be rotated;
/// its longest edge sets the axes. Returns None if any edge isn't at a right angle to it.
pub fn rect_cover(ring: &[Pt]) -> Option<Vec<Vec<Pt>>> {
    let n = ring.len();
    if n < 4 {
        return None;
    }
    let longest = (0..n).max_by(|&i, &j| {
        ring[i]
            .dist(ring[(i + 1) % n])
            .total_cmp(&ring[j].dist(ring[(j + 1) % n]))
    })?;
    let d = ring[(longest + 1) % n].sub(ring[longest]).norm();
    let (cos, sin) = (d.x, d.y);
    // Into axis-aligned coordinates (rotate by -θ).
    let local: Vec<Pt> = ring.iter().map(|p| rotate(*p, cos, -sin)).collect();
    for i in 0..n {
        let e = local[(i + 1) % n].sub(local[i]);
        if e.x.abs() > 0.5 && e.y.abs() > 0.5 {
            return None;
        }
    }
    let mut xs: Vec<f64> = local.iter().map(|p| p.x).collect();
    let mut ys: Vec<f64> = local.iter().map(|p| p.y).collect();
    for v in [&mut xs, &mut ys] {
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    }
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    if nx == 0 || ny == 0 {
        return None;
    }
    let inside: Vec<Vec<bool>> = (0..nx)
        .map(|i| {
            (0..ny)
                .map(|j| {
                    let c = Pt::new((xs[i] + xs[i + 1]) / 2.0, (ys[j] + ys[j + 1]) / 2.0);
                    point_in_ring(c, &local)
                })
                .collect()
        })
        .collect();
    let mut covered = vec![vec![false; ny]; nx];
    let full = |i0: usize, i1: usize, j: usize| (i0..=i1).all(|i| inside[i][j]);
    let full_col = |j0: usize, j1: usize, i: usize| (j0..=j1).all(|j| inside[i][j]);
    let mut rects: Vec<(usize, usize, usize, usize)> = vec![];
    for j in 0..ny {
        for i in 0..nx {
            if !inside[i][j] || covered[i][j] {
                continue;
            }
            // Widest-first and tallest-first maximal rectangles through the cell.
            let grow_x = |i: usize, j: usize| {
                let (mut a, mut b) = (i, i);
                while a > 0 && inside[a - 1][j] {
                    a -= 1;
                }
                while b + 1 < nx && inside[b + 1][j] {
                    b += 1;
                }
                (a, b)
            };
            let (a, b) = grow_x(i, j);
            let (mut c, mut e) = (j, j);
            while c > 0 && full(a, b, c - 1) {
                c -= 1;
            }
            while e + 1 < ny && full(a, b, e + 1) {
                e += 1;
            }
            let wide = (a, b, c, e);
            let (mut c2, mut e2) = (j, j);
            while c2 > 0 && inside[i][c2 - 1] {
                c2 -= 1;
            }
            while e2 + 1 < ny && inside[i][e2 + 1] {
                e2 += 1;
            }
            let (mut a2, mut b2) = (i, i);
            while a2 > 0 && full_col(c2, e2, a2 - 1) {
                a2 -= 1;
            }
            while b2 + 1 < nx && full_col(c2, e2, b2 + 1) {
                b2 += 1;
            }
            let tall = (a2, b2, c2, e2);
            let area =
                |r: (usize, usize, usize, usize)| (xs[r.1 + 1] - xs[r.0]) * (ys[r.3 + 1] - ys[r.2]);
            // Prefer the one covering more still-uncovered cells, then the larger.
            let fresh = |r: (usize, usize, usize, usize)| {
                (r.0..=r.1)
                    .flat_map(|x| (r.2..=r.3).map(move |y| (x, y)))
                    .filter(|(x, y)| !covered[*x][*y])
                    .count()
            };
            let pick = if (fresh(wide), area(wide) as i64) >= (fresh(tall), area(tall) as i64) {
                wide
            } else {
                tall
            };
            for row in covered.iter_mut().take(pick.1 + 1).skip(pick.0) {
                for c in row.iter_mut().take(pick.3 + 1).skip(pick.2) {
                    *c = true;
                }
            }
            rects.push(pick);
        }
    }
    Some(
        rects
            .into_iter()
            .map(|(a, b, c, e)| {
                [
                    Pt::new(xs[a], ys[c]),
                    Pt::new(xs[b + 1], ys[c]),
                    Pt::new(xs[b + 1], ys[e + 1]),
                    Pt::new(xs[a], ys[e + 1]),
                ]
                .iter()
                .map(|p| rotate(*p, cos, sin))
                .collect()
            })
            .collect(),
    )
}

/// `a` minus every polygon in `cut`.
pub fn difference(a: &Poly, cut: &[Poly]) -> Vec<Poly> {
    use geo::BooleanOps;
    let mut acc = geo::MultiPolygon::new(vec![to_geo(a)]);
    for c in cut {
        if c.outer.len() >= 3 && signed_area(&c.outer).abs() >= 1.0 {
            acc = acc.difference(&geo::MultiPolygon::new(vec![to_geo(c)]));
        }
    }
    acc.0
        .iter()
        .map(|g| Poly {
            outer: from_geo_ring(g.exterior()),
            holes: g.interiors().iter().map(from_geo_ring).collect(),
        })
        .filter(|p| p.outer.len() >= 3 && p.area() > 1.0)
        .collect()
}

/// Axis-aligned bounds of points.
pub fn bounds_of(pts: &[Pt]) -> Option<(Pt, Pt)> {
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Pt::new(lo.x.min(p.x), lo.y.min(p.y)),
            Pt::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

/// Contour segments at height `level` of a grid of values (marching squares). Segments are
/// in grid coordinates (i, j fractional); `value(i, j)` gives a node's height.
pub fn contour_segments(
    nx: usize,
    ny: usize,
    value: impl Fn(usize, usize) -> f64,
    level: f64,
) -> Vec<[Pt; 2]> {
    let mut out = vec![];
    if nx < 2 || ny < 2 {
        return out;
    }
    // Where the level crosses the edge between two nodes.
    let cross = |a: Pt, va: f64, b: Pt, vb: f64| a.lerp(b, (level - va) / (vb - va));
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            // Corners counter-clockwise from the lower left; nudge exact hits off the level.
            let v = |x: usize, y: usize| {
                let z = value(x, y);
                if (z - level).abs() < 1e-9 {
                    z + 1e-6
                } else {
                    z
                }
            };
            let c = [
                (Pt::new(i as f64, j as f64), v(i, j)),
                (Pt::new(i as f64 + 1.0, j as f64), v(i + 1, j)),
                (Pt::new(i as f64 + 1.0, j as f64 + 1.0), v(i + 1, j + 1)),
                (Pt::new(i as f64, j as f64 + 1.0), v(i, j + 1)),
            ];
            let mut hits: Vec<Pt> = vec![];
            for k in 0..4 {
                let (a, va) = c[k];
                let (b, vb) = c[(k + 1) % 4];
                if (va > level) != (vb > level) {
                    hits.push(cross(a, va, b, vb));
                }
            }
            match hits.len() {
                2 => out.push([hits[0], hits[1]]),
                4 => {
                    // A saddle: pair by the cell's center value.
                    let center = (c[0].1 + c[1].1 + c[2].1 + c[3].1) / 4.0;
                    if (center > level) == (c[0].1 > level) {
                        out.push([hits[0], hits[3]]);
                        out.push([hits[1], hits[2]]);
                    } else {
                        out.push([hits[0], hits[1]]);
                        out.push([hits[2], hits[3]]);
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Convex hull of points (counter-clockwise, Andrew's monotone chain).
pub fn convex_hull(points: &[Pt]) -> Vec<Pt> {
    let mut p: Vec<Pt> = points.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup_by(|a, b| a.dist(*b) < 1e-9);
    if p.len() < 3 {
        return p;
    }
    let cross = |o: Pt, a: Pt, b: Pt| a.sub(o).cross(b.sub(o));
    let mut lower: Vec<Pt> = vec![];
    for q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *q) <= 0.0 {
            lower.pop();
        }
        lower.push(*q);
    }
    let mut upper: Vec<Pt> = vec![];
    for q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *q) <= 0.0 {
            upper.pop();
        }
        upper.push(*q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Like [`Prism::triangles`], but the top follows `top(p)` at each vertex (a wall under a
/// sloped roof).
pub fn prism_triangles_to(base: &Poly, z0: f64, top: impl Fn(Pt) -> f64) -> Vec<f32> {
    let mut out = vec![];
    let (verts, tris) = triangulate(base);
    let ccw = signed_area(&base.outer) > 0.0;
    let mut push = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
        for v in [a, b, c] {
            out.extend_from_slice(&[v[0] as f32, v[1] as f32, v[2] as f32]);
        }
    };
    let v3 = |p: Pt, z: f64| [p.x, p.y, z];
    for t in &tris {
        let (a, b, c) = (verts[t[0]], verts[t[1]], verts[t[2]]);
        let (b, c) = if b.sub(a).cross(c.sub(a)) > 0.0 {
            (b, c)
        } else {
            (c, b)
        };
        push(v3(a, top(a)), v3(b, top(b)), v3(c, top(c)));
        push(v3(a, z0), v3(c, z0), v3(b, z0));
    }
    let mut side = |ring: &[Pt], outward_ccw: bool| {
        let n = ring.len();
        for i in 0..n {
            let (mut a, mut b) = (ring[i], ring[(i + 1) % n]);
            if !outward_ccw {
                std::mem::swap(&mut a, &mut b);
            }
            push(v3(a, z0), v3(b, z0), v3(b, top(b)));
            push(v3(a, z0), v3(b, top(b)), v3(a, top(a)));
        }
    };
    side(&base.outer, ccw);
    for h in &base.holes {
        side(h, signed_area(h) < 0.0);
    }
    out
}

/// The 8 corners of a box of cross-section `w` × `h` (horizontal × vertical) centered on
/// the 3D segment `a` → `b` (not vertical), in order: start face, then end face.
pub fn box_corners(a: [f64; 3], b: [f64; 3], w: f64, h: f64) -> [[f64; 3]; 8] {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let dl = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
    let d = [d[0] / dl, d[1] / dl, d[2] / dl];
    // Horizontal side vector, then up = d × s.
    let sl = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
    let s = [-d[1] / sl, d[0] / sl, 0.0];
    let u = [
        d[1] * s[2] - d[2] * s[1],
        d[2] * s[0] - d[0] * s[2],
        d[0] * s[1] - d[1] * s[0],
    ];
    let mut out = [[0.0; 3]; 8];
    let offs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for (k, base) in [a, b].iter().enumerate() {
        for (i, (so, uo)) in offs.iter().enumerate() {
            out[k * 4 + i] = [
                base[0] + s[0] * so * w / 2.0 + u[0] * uo * h / 2.0,
                base[1] + s[1] * so * w / 2.0 + u[1] * uo * h / 2.0,
                base[2] + s[2] * so * w / 2.0 + u[2] * uo * h / 2.0,
            ];
        }
    }
    out
}

/// Triangles (9 floats each) of a box from [`box_corners`].
pub fn box_triangles(c: &[[f64; 3]; 8]) -> Vec<f32> {
    const FACES: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut out = vec![];
    for f in FACES {
        for tri in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
            for i in tri {
                out.extend_from_slice(&[c[i][0] as f32, c[i][1] as f32, c[i][2] as f32]);
            }
        }
    }
    out
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    #[test]
    fn convexity() {
        let sq = [
            Pt::new(0.0, 0.0),
            Pt::new(1.0, 0.0),
            Pt::new(1.0, 1.0),
            Pt::new(0.0, 1.0),
        ];
        assert!(is_convex(&sq));
        let l = [
            Pt::new(0.0, 0.0),
            Pt::new(2.0, 0.0),
            Pt::new(2.0, 1.0),
            Pt::new(1.0, 1.0),
            Pt::new(1.0, 2.0),
            Pt::new(0.0, 2.0),
        ];
        assert!(!is_convex(&l));
    }

    #[test]
    fn l_shape_is_covered_by_two_overlapping_rectangles() {
        let l = [
            Pt::new(0.0, 0.0),
            Pt::new(12000.0, 0.0),
            Pt::new(12000.0, 5000.0),
            Pt::new(5000.0, 5000.0),
            Pt::new(5000.0, 11000.0),
            Pt::new(0.0, 11000.0),
        ];
        let r = rect_cover(&l).unwrap();
        assert_eq!(r.len(), 2);
        let areas: Vec<f64> = r.iter().map(|p| signed_area(p).abs()).collect();
        assert!(areas.contains(&(12000.0 * 5000.0)));
        assert!(
            areas.contains(&(5000.0 * 11000.0)),
            "the wing runs through the main block"
        );
        // A 45° rotated L works too; a triangle doesn't.
        let rot: Vec<Pt> = l
            .iter()
            .map(|p| rotate(*p, 0.5f64.sqrt(), 0.5f64.sqrt()))
            .collect();
        assert_eq!(rect_cover(&rot).unwrap().len(), 2);
        let tri = [Pt::new(0.0, 0.0), Pt::new(5.0, 0.0), Pt::new(0.0, 5.0)];
        assert!(rect_cover(&tri).is_none());
    }

    #[test]
    fn u_and_t_shapes() {
        // U: three rectangles (two legs through the base).
        let u = [
            Pt::new(0.0, 0.0),
            Pt::new(9000.0, 0.0),
            Pt::new(9000.0, 8000.0),
            Pt::new(6000.0, 8000.0),
            Pt::new(6000.0, 3000.0),
            Pt::new(3000.0, 3000.0),
            Pt::new(3000.0, 8000.0),
            Pt::new(0.0, 8000.0),
        ];
        assert_eq!(rect_cover(&u).unwrap().len(), 3);
    }

    #[test]
    fn difference_hull_and_boxes() {
        let big = Poly::simple(vec![
            Pt::new(0.0, 0.0),
            Pt::new(10.0, 0.0),
            Pt::new(10.0, 10.0),
            Pt::new(0.0, 10.0),
        ]);
        let hole = Poly::simple(vec![
            Pt::new(2.0, 2.0),
            Pt::new(4.0, 2.0),
            Pt::new(4.0, 4.0),
            Pt::new(2.0, 4.0),
        ]);
        let d = difference(&big, &[hole]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].holes.len(), 1);
        assert!((d[0].area() - 96.0).abs() < 1e-9);
        let h = convex_hull(&[
            Pt::new(0.0, 0.0),
            Pt::new(1.0, 1.0),
            Pt::new(2.0, 0.0),
            Pt::new(1.0, 3.0),
            Pt::new(1.0, 0.5),
        ]);
        assert_eq!(h.len(), 3);
        let c = box_corners([0.0, 0.0, 0.0], [1000.0, 0.0, 500.0], 50.0, 40.0);
        assert!((c[1][1] - 25.0).abs() < 1e-9, "side vector is horizontal");
        assert_eq!(box_triangles(&c).len(), 12 * 9);
    }
}

/// Crate version from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Poly {
        Poly::simple(vec![
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ])
    }

    #[test]
    fn line_intersection_of_axes() {
        let p = line_intersection(
            Pt::new(0.0, 5.0),
            Pt::new(1.0, 0.0),
            Pt::new(3.0, 0.0),
            Pt::new(0.0, 1.0),
        );
        assert_eq!(p, Some(Pt::new(3.0, 5.0)));
        assert!(line_intersection(
            Pt::default(),
            Pt::new(1.0, 0.0),
            Pt::new(0.0, 1.0),
            Pt::new(2.0, 0.0)
        )
        .is_none());
    }

    #[test]
    fn union_of_overlapping_rects_is_one_polygon() {
        let u = union_all(&[rect(0.0, 0.0, 10.0, 10.0), rect(5.0, 0.0, 15.0, 10.0)]);
        assert_eq!(u.len(), 1);
        assert!((u[0].area() - 150.0).abs() < 1.0);
        assert_eq!(u[0].outer.len(), 4);
    }

    #[test]
    fn union_of_ring_of_rects_has_hole() {
        // Four 1 mm-thick bars forming a 10 × 10 frame.
        let u = union_all(&[
            rect(0.0, 0.0, 10.0, 1.0),
            rect(9.0, 0.0, 10.0, 10.0),
            rect(0.0, 9.0, 10.0, 10.0),
            rect(0.0, 0.0, 1.0, 10.0),
        ]);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].holes.len(), 1);
        assert!((u[0].area() - 36.0).abs() < 1.0);
        assert!(u[0].contains(Pt::new(0.5, 5.0)));
        assert!(!u[0].contains(Pt::new(5.0, 5.0)));
    }

    #[test]
    fn prism_triangle_count() {
        let p = Prism {
            base: rect(0.0, 0.0, 1.0, 1.0),
            z0: 0.0,
            z1: 1.0,
        };
        // 2 caps × 2 triangles + 4 sides × 2 triangles = 12 triangles × 9 floats.
        assert_eq!(p.triangles().len(), 12 * 9);
    }

    #[test]
    fn projection_to_segment() {
        let (t, d) = project_to_segment(Pt::new(5.0, 3.0), Pt::new(0.0, 0.0), Pt::new(10.0, 0.0));
        assert!((t - 0.5).abs() < 1e-12);
        assert!((d - 3.0).abs() < 1e-12);
    }
}
#[cfg(test)]
mod union_edge_cases {
    use super::*;

    #[test]
    fn mitered_wall_quads_sharing_an_edge_merge() {
        let w1 = Poly::simple(vec![
            Pt::new(-101.6, -101.6),
            Pt::new(12293.6, -101.6),
            Pt::new(12090.4, 101.6),
            Pt::new(101.6, 101.6),
        ]);
        let w2 = Poly::simple(vec![
            Pt::new(12293.6, -101.6),
            Pt::new(12293.6, 9245.6),
            Pt::new(12090.4, 9042.4),
            Pt::new(12090.4, 101.6),
        ]);
        let u = union_all(&[w1.clone(), w2.clone()]);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].outer.len(), 6);
        assert!((u[0].area() - (w1.area() + w2.area())).abs() < 1000.0);
    }

    #[test]
    fn offset_square_grows_each_side() {
        let sq = vec![
            Pt::new(0.0, 0.0),
            Pt::new(10.0, 0.0),
            Pt::new(10.0, 10.0),
            Pt::new(0.0, 10.0),
        ];
        let o = offset_ring(&sq, 1.0);
        assert!(o.iter().any(|p| p.dist(Pt::new(-1.0, -1.0)) < 1e-9));
        assert!((signed_area(&o) - 144.0).abs() < 1e-9);
        let mut cw = sq.clone();
        cw.reverse();
        assert!((signed_area(&offset_ring(&cw, 1.0)).abs() - 144.0).abs() < 1e-9);
    }
}
