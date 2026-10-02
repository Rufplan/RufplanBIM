//! Precedents for Generate (ADR-099): famous architects and houses, each as the design
//! moves Claude can make in the building plan's own terms (massing, outdoor rooms, roofs,
//! cladding, glazing, fascia), so "in the manner of Neutra" builds like Neutra.

use serde::Serialize;
use ts_rs::TS;

/// An architect or work to design after.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Precedent {
    pub name: String,
    /// "Architect" or "Work".
    pub group: String,
    /// The moves, in the plan's terms.
    pub moves: String,
}

/// (name, group, moves).
const PRECEDENTS: &[(&str, &str, &str)] = &[
    (
        "Frank Lloyd Wright (Prairie)",
        "Architect",
        "Long, low and horizontal: a two-story core with one-story wings and terraces reaching into the garden (cross or T plan). Hip roofs at 3:12 with very deep 4-6 ft overhangs on every story. Brick base (masonry-red-brick or masonry-common-brick) with stucco (plaster-stucco-sand) above; ribbon glazing in bands, Prairie grilles, casements; a central hearth and stair; a covered porch at the entry.",
    ),
    (
        "Frank Lloyd Wright (Usonian)",
        "Architect",
        "One story on an L plan round a garden terrace; flat roofs with 3-4 ft overhangs and a thin fascia; board walls (siding-cedar-lap-stained or siding-channel-rustic) and brick (masonry-common-brick); window walls and sliding doors to the terrace, ribbon glazing on the street; a carport-like garage at the front.",
    ),
    (
        "Fallingwater (Wright)",
        "Work",
        "Stacked, offset horizontal trays: each story cantilevers 6-12 ft past the one below in a different direction, with decks on the roofs below; flat roofs, 3-4 ft overhangs; a stone core (stone-fieldstone) and buff stucco (plaster-stucco-sand) bands; ribbon glazing and window walls; terraces at grade; cantileverColumns false.",
    ),
    (
        "Robie House (Wright)",
        "Work",
        "A long two-volume plan: a long main floor with a narrower upper volume set back; very low hip roofs (2.5:12) with 5-6 ft overhangs; roman-brick base (masonry-red-brick), ribbon casements with Prairie grilles, terraces along the south.",
    ),
    (
        "Mies van der Rohe",
        "Architect",
        "A pure one-story rectangle or two offset rectangles; flat roof with a thin dark metal edge fascia and 2-4 ft overhang; window walls on every side but a solid service core (bath, kitchen, mechanical) in the middle; a terrace or porch at one end under the roof; steel; white or travertine paving; cantileverColumns true.",
    ),
    (
        "Farnsworth House (Mies)",
        "Work",
        "One rectangle, one room deep: window walls all round, a central core of bath and kitchen, flat roof with a thin metal edge, a covered porch at one end and a terrace below it, travertine paving (stone-travertine), white.",
    ),
    (
        "Glass House (Philip Johnson)",
        "Work",
        "A single rectangle with window walls on all four sides, a brick-clad bath core (masonry-common-brick), flat roof with a thin black edge, no overhang, a lawn terrace.",
    ),
    (
        "Le Corbusier",
        "Architect",
        "The five points: an upper story larger than the ground floor, cantilevered on columns (cantileverColumns true) over a recessed entry and garage; ribbon glazing in long bands; free plan; a roof deck on the top; white stucco (plaster-stucco-white); flat roofs, no overhang.",
    ),
    (
        "Villa Savoye (Le Corbusier)",
        "Work",
        "A square upper story raised over a smaller curved-back ground floor (garage, entry, service): cantileverColumns true; ribbon glazing round the upper story, a deck (roof terrace) cut into it; flat roof, white stucco.",
    ),
    (
        "Richard Neutra",
        "Architect",
        "A pinwheel of wings off a central core, flat roofs at different heights with deep thin overhangs (3-5 ft) running past the walls; window walls and sliding doors opening every room onto terraces; white stucco (plaster-stucco-white) with stone walls (stone-ledgestone) and wood soffits; desert or garden landscape.",
    ),
    (
        "Kaufmann Desert House (Neutra)",
        "Work",
        "Four wings pinwheeling from a living core, one story with a small upper loggia (deck); flat roofs, 4-6 ft overhangs, thin fascia; window walls and sliding doors; stone (stone-ledgestone) and white; desert planting (Palo Verde, Honey Mesquite, agave, grasses).",
    ),
    (
        "Pierre Koenig / Stahl House",
        "Work",
        "An L plan wrapped round a terrace and view: the long arms all window walls and sliding doors, the street side solid; one story, flat roof cantilevering 5-6 ft past the glass, steel, a thin fascia; terrace paving.",
    ),
    (
        "Charles & Ray Eames / Eames House",
        "Work",
        "Two rectangular volumes (house and studio) in a line with a courtyard between; a steel frame; flat roofs, no overhang; walls of ribbon and window-wall glazing with solid colored panels; two-story living space.",
    ),
    (
        "Joseph Eichler (mid-century)",
        "Architect",
        "One story on an L or U plan round an atrium courtyard at the entry; very low gable (1.5:12) or flat roof with 2-3 ft overhangs; a blank street face (glazing none) with the garage forward; window walls and sliding doors to the back garden; vertical cedar siding (siding-cedar-vertical).",
    ),
    (
        "Marcel Breuer",
        "Architect",
        "Binuclear: a living wing and a sleeping wing joined by an entry link; butterfly or flat roofs; fieldstone walls (stone-fieldstone) and vertical cedar; a cantilevered deck; window walls to the garden, ribbon elsewhere.",
    ),
    (
        "Gropius House (Walter Gropius)",
        "Work",
        "A compact two-story box made dynamic by a screened porch wing and an entry canopy (porch); flat roofs; white painted vertical boards (siding-board-batten-white); ribbon glazing; a roof deck off the bedrooms.",
    ),
    (
        "Rudolph Schindler",
        "Architect",
        "Interlocking L-shaped wings round courtyards, each room opening to its own garden; flat roofs at different heights; concrete (concrete-board-formed) and redwood (siding-cedar-vertical); sliding doors and window walls; decks on the roofs.",
    ),
    (
        "Tadao Ando",
        "Architect",
        "Board-formed concrete (concrete-board-formed) inside and out; two parallel bars with a courtyard between, or a bar and a wall; flat roofs, no overhang (overhang 0), no fascia; the street face blank, window walls only to the courtyard; precise, minimal.",
    ),
    (
        "Koshino House (Ando)",
        "Work",
        "Two parallel concrete bars of different lengths, one two stories, joined below grade by a corridor, with a terraced courtyard between; flat roofs, no overhang; window walls to the courtyard only.",
    ),
    (
        "Glenn Murcutt",
        "Architect",
        "A long, thin one-story bar one room deep, open along its view side; shed or butterfly roofs of standing-seam metal (metal-standing-seam), 2-3 ft overhangs; corrugated metal or timber walls; window walls and sliding doors along the long side, ribbon high on the other; decks at the ends.",
    ),
    (
        "Richard Meier",
        "Architect",
        "White (plaster-stucco-white or metal panels): a solid, closed entry side and an open view side of window walls; two or three stories with decks and a cantilevered upper volume; flat roofs; a ramp or bridge to the entry porch.",
    ),
    (
        "Alvar Aalto",
        "Architect",
        "An L plan round a garden court; white stucco and vertical wood (siding-cedar-vertical), stone at the base; flat roofs, one wing with a sedum roof (roof-sedum); varied glazing: window walls to the court, ribbon and punched elsewhere.",
    ),
    (
        "Greene & Greene (Craftsman)",
        "Architect",
        "Spreading two-story Craftsman: several low gables (4:12) with 3 ft overhangs, a broad entry porch, sleeping porches as upper decks; cedar shingles (siding-cedar-shingles) on a fieldstone base; Craftsman grilles; terraces.",
    ),
    (
        "Sea Ranch (MLTW)",
        "Work",
        "Compact two-story volumes with steep shed roofs (8:12) of different heights sloping the same way, no overhang; weathered cedar (siding-cedar-bevel-weathered) inside and out; window walls to the view; a courtyard sheltered from the wind.",
    ),
    (
        "Olson Kundig",
        "Architect",
        "Rugged and big-windowed: a steel and concrete base with a cantilevered wood upper volume; shed roofs; huge window walls and folding doors opening living spaces to decks and terraces; corten (metal-corten), board-formed concrete and dark cedar.",
    ),
    (
        "Peter Zumthor",
        "Architect",
        "Monolithic, quiet volumes: one material everywhere (concrete-board-formed or dark wood siding-shou-sugi-ban); simple gable or flat volumes with no overhang; few, large deep-set windows (window walls in chosen rooms, glazing none elsewhere).",
    ),
    (
        "Bjarke Ingels (BIG)",
        "Architect",
        "Stepped, sculpted massing: stories stepping back to form a stair of decks; green roofs (roof-sedum); window walls on the stepped faces; one material wrapping walls and roofs.",
    ),
    (
        "Kengo Kuma",
        "Architect",
        "Wood in layers: vertical cedar (siding-cedar-vertical or siding-accoya-slats) screening the walls; a long porch (engawa) along the garden side; shed or low gable roofs with deep thin overhangs; window walls and sliding doors behind the screens; a courtyard garden.",
    ),
    (
        "Luis Barragán",
        "Architect",
        "Colored stucco walls (plaster-stucco-sand, plaster-venetian) enclosing courtyards and terraces; flat roofs, no overhang; the street face nearly blank; a few large windows; roof terraces as decks; water and garden.",
    ),
    (
        "Paul Rudolph (Sarasota)",
        "Architect",
        "Florida modern: a light one-story pavilion; butterfly or flat roofs with deep overhangs; sliding doors and window walls on both long sides for cross-breeze; a screened porch; terrace paving.",
    ),
    (
        "Louis Kahn",
        "Architect",
        "Two square volumes set at an angle or offset, one for living (double-height) and one for sleeping; cypress or cedar (siding-cedar-vertical) on a stone base; deep-set window walls in chosen bays, solid elsewhere; flat roofs, no overhang.",
    ),
];

/// The precedents, for the Generate dialog.
#[tauri::command]
pub fn generate_precedents() -> Vec<Precedent> {
    all()
}

pub fn all() -> Vec<Precedent> {
    PRECEDENTS
        .iter()
        .map(|(name, group, moves)| Precedent {
            name: (*name).into(),
            group: (*group).into(),
            moves: (*moves).into(),
        })
        .collect()
}

/// The moves for a precedent named in the brief (exactly, ignoring case), if listed.
pub fn moves_of(name: &str) -> Option<&'static str> {
    let n = name.trim().to_lowercase();
    PRECEDENTS
        .iter()
        .find(|(p, _, _)| p.to_lowercase() == n)
        .map(|(_, _, m)| *m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_a_precedent_names_is_in_the_library() {
        let ids: Vec<String> = studio_core::library::library()
            .into_iter()
            .map(|p| p.id)
            .collect();
        for p in all() {
            for word in p
                .moves
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            {
                let looks_like_id = word.contains('-')
                    && [
                        "masonry", "plaster", "siding", "stone", "metal", "concrete", "roof",
                        "site", "wood",
                    ]
                    .iter()
                    .any(|k| word.starts_with(&format!("{k}-")));
                if looks_like_id {
                    assert!(ids.contains(&word.to_string()), "{}: {word}", p.name);
                }
            }
        }
        assert!(all().len() >= 30);
        assert!(moves_of("richard neutra").is_some());
    }
}
