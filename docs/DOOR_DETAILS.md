# Door Details — Sources (ADR-122)

The Detail Library's door details: the head, jamb and sill of each door, plus the meeting
stile of a pair, the interlock of a slider or the fold joint of a bi-fold. Each one is redrawn
from the dimensions in the document listed, not copied from the maker's CAD. Inserted, a
detail becomes a drafting view of ordinary lines, filled regions, detail components and
notes, so it can be edited like any other.

Every manufacturer detail carries three lines of text under the drawing: its source, the
source's URL, and a note to check it against the maker's current details. Where a source
doesn't state a size, the detail is drawn at a typical size and a fourth line names those
sizes ("Not stated in the source…", the last column below). Verify those sizes before you
issue a drawing.

The door data is in `crates/studio-core/src/details/doors/catalog.rs` and the drawing engine
is in `doors.rs`. Change a door's sizes there and all of its details follow.

| Door | Maker | Category | Source | Drawn at typical sizes |
|---|---|---|---|---|
| Exterior Door - Wood Frame & Siding | Typical | Typical | [ASTM E2112 flashing sequence per Building America Solution Center; IRC R703.4](https://basc.pnnl.gov/resource-guides/windows-and-doors-are-fully-flashed) |  |
| Exterior Door - Brick Veneer | Typical | Typical | [IRC R703.8 (lintels, weeps 33" o.c. max.); BIA stepped flashing guide](https://www.gobrick.com/media/file/stepped-flashing.pdf) |  |
| Exterior Door - Stucco | Typical | Typical | [ASTM E2112; ASTM C926/C1063 stucco and casing beads](https://basc.pnnl.gov/resource-guides/windows-and-doors-are-fully-flashed) |  |
| Exterior Door - Accessible Flush Sill | Typical | Typical | [2010 ADA Standards 303 and 404.2.5; IRC R311.3.1](https://www.access-board.gov/ada/) |  |
| Hollow Metal Door - Metal Stud Partition | Typical | Typical | [SDI-111 Recommended Standard Details for Steel Doors and Frames](https://steeldoor.org/wp-content/uploads/2020/02/SDI_111.pdf) |  |
| Hollow Metal Door - CMU Exterior | Typical | Typical | [SDI-111 frame details; SDI-110 masonry coordination](https://steeldoor.org/wp-content/uploads/2020/02/SDI_111.pdf) |  |
| Interior Door - Prehung Wood | Typical | Typical | [Prehung interior door practice: 4-9/16" jamb for 2x4 walls; 1/8" margins at head and strike](https://www.doornmore.com/help/jamb-width.html) |  |
| Marvin Ultimate Inswing French Door G2 | Marvin | Hinged Patio | [Ultimate Inswing French Door G2 Architectural Detail Manual](MARVIN_IN) |  |
| Marvin Ultimate Outswing French Door G2 | Marvin | Hinged Patio | [Ultimate Outswing French Door G2 Architectural Detail Manual](MARVIN_OUT) |  |
| Marvin Elevate Inswing French Door | Marvin | Hinged Patio | [Elevate Inswing French Door Architectural Detail Manual](https://marvin.com/f/1019562/x/971560f611/marvin-elevate-inswing-french-door-adm.pdf) | SILL HEIGHT READ FROM THE SECTION'S 1-5/8" LABEL |
| Andersen A-Series Hinged Inswing Patio Door | Andersen | Hinged Patio | [A-Series Product Guide, hinged inswing patio door sections](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/2020-to-current-literature/a-series-product-guide.pdf) | PANEL THICKNESS 1-3/4", FRAME PROJECTION PAST FLANGE 1" |
| Andersen A-Series Hinged Outswing Patio Door | Andersen | Hinged Patio | [A-Series Product Guide, hinged outswing patio door sections](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/2020-to-current-literature/a-series-product-guide.pdf) | PANEL THICKNESS 1-3/4", FRAME PROJECTION PAST FLANGE 1" |
| Andersen E-Series Hinged Inswing Patio Door | Andersen | Hinged Patio | [E-Series Product Guide, hinged patio doors](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/2020-to-current-literature/e-series-product-guide.pdf) | FRAME THICKNESS, PANEL 1-3/4", GLASS 3/4", SILL HEIGHT (SECTIONS ARE IMAGE-ONLY) |
| Pella Reserve Traditional In-Swing Patio Door | Pella | Hinged Patio | [Pella Reserve In-Swing Door Product Selection Guide (unit sections)](https://media.pella.com/professional/adm/Clad-Wood/Pella-Reserve_InSwingDoor.pdf) | SILL HEIGHT READ FROM THE SECTION'S 1-13/16" LABEL |
| Pella Reserve Traditional Out-Swing Patio Door | Pella | Hinged Patio | [Pella Reserve Traditional Clad Out-Swing Door product description and sections](https://media.pella.com/professional/adm/Clad-Wood/Pella-ReserveTrad-Clad-OS_DPD.rtf) | SILL HEIGHT READ FROM THE SECTION'S 1-5/8" LABEL |
| Sierra Pacific Clad Inswing Door | Sierra Pacific Windows | Hinged Patio | [Clad Inswing Door Architectural Design Manual](https://sierrapacificwindows.com/Technical/Door/Swinging/Aluminum%20Clad%20Wood/Inswing%20Door/Architectural%20Design%20Manual/Clad%20Inswing%20Door%20ADM.pdf) | SILL HEIGHT 1-1/2"; RAILS FROM THE 3-5/8"/4-5/8"/6-3/8" OPTIONS |
| Weather Shield Signature French Door (Inswing) | Weather Shield | Hinged Patio | [Signature Series French Door Quick Spec and sections SG_8510](https://weathershield.com/wp-content/uploads/2022/01/SG_8510_FD_QS.pdf) | STILES, FRAME THICKNESS AND PROJECTION, GLASS, SILL HEIGHT |
| Loewen Inswing French Terrace Door | Loewen | Hinged Patio | [Loewen Swinging Terrace & French Doors guide and 2-1/4" Inswing technical sheet](https://www.loewen.com/wp-content/uploads/2025/08/Loewen-SwingingTerraceFrenchDoors-v3-3.pdf) | FRAME PROFILE, GLASS THICKNESS, SILL HEIGHT TAKEN AS THE 2-1/4" SILL |
| Marvin Modern Multi-Slide Door | Marvin | Sliding & Folding | [Modern Multi-Slide Door Architectural Detail Manual and Site Preparation 19915907](https://www.marvin.com/f/1019562/x/8e71dd93a8/marvin-modern-multi-slide-door-automated-adm.pdf) |  |
| Marvin Modern Multi-Slide Door (HP Sill in Slot) | Marvin | Sliding & Folding | [Modern Multi-Slide Door Architectural Detail Manual, HP sill slot recess table](https://www.marvin.com/f/1019562/x/8e71dd93a8/marvin-modern-multi-slide-door-automated-adm.pdf) |  |
| Marvin Ultimate Multi-Slide Door | Marvin | Sliding & Folding | [Ultimate Multi-Slide Door Architectural Detail Manual](https://marvin.com/f/1019562/x/95ea864362/adm-ultimate-multislide-door.pdf) | FRAME PROJECTION 1"; JAMB DEPTH READ FROM A GARBLED TABLE |
| Marvin Ultimate Lift and Slide Door | Marvin | Sliding & Folding | [Ultimate Lift and Slide Door Architectural Detail Manual, slot depth charts](https://www.marvin.com/f/1019562/x/0ec1712b0c/adm-ultimate-lift-and-slide-door.pdf) | FRAME PROJECTION 1" |
| Marvin Ultimate Sliding French Door | Marvin | Sliding & Folding | [Ultimate Sliding Door Architectural Detail Manual](https://marvin.com/f/1019562/x/25756ad0ad/ultimate-sliding-door-adm.pdf) | FRAME PROJECTION 1"; SILL HEIGHT READ FROM A 1-15/16" LABEL |
| Marvin Ultimate Bi-Fold Door | Marvin | Sliding & Folding | [Ultimate Bi-Fold Door Architectural Detail Manual](https://marvin.com/f/1019562/x/907ebaffed/marvin-ultimate-bifold-door-adm.pdf) | FRAME PROJECTION 1" |
| Marvin Ultimate Bi-Fold Door (Low Profile Sill) | Marvin | Sliding & Folding | [Ultimate Bi-Fold Door Architectural Detail Manual](https://marvin.com/f/1019562/x/907ebaffed/marvin-ultimate-bifold-door-adm.pdf) | FRAME PROJECTION 1" |
| Western Series 7600 Multi-Slide Door | Western Architectural Openings (Western Window Systems) | Sliding & Folding | [7600 Multi-Slide Door Product Data Sheet 7600-MSD-PDS-260501](https://westernao.com/sites/default/files/2026-06/document_ae6kipaadh1ep6qguj84svht16_0.pdf) | PANEL DEPTH, FRAME THICKNESS, RAILS (STILE TAKEN AS THE 3.58" FIXED CONDITION) |
| Western Series 7600 Multi-Slide Door (Flush Sill) | Western Architectural Openings (Western Window Systems) | Sliding & Folding | [7600 Multi-Slide Door Product Data Sheet, flush threshold (0.75" flooring condition)](https://westernao.com/sites/default/files/2026-06/document_ae6kipaadh1ep6qguj84svht16_0.pdf) | PANEL DEPTH, FRAME THICKNESS, RAILS |
| Western Series 600 Multi-Slide Door | Western Architectural Openings (Western Window Systems) | Sliding & Folding | [600 Multi-Slide Door Product Data Sheet 600-MSD-PDS-260501](https://westernao.com/sites/default/files/2026-06/document_v3g6dtiq091ojd0r0jc7pf4t3q.pdf) | PANEL DEPTH, FRAME THICKNESS, RAILS |
| Pella Reserve Contemporary Multi-Slide Door | Pella | Sliding & Folding | [Pella Reserve Contemporary Multi-Slide Door ADM (design data formulas)](https://media.pella.com/professional/adm/Specialty/Pella-ReserveCont_MultiSlideDoor.pdf) | PANEL DEPTH, RAILS |
| Andersen MultiGlide Door | Andersen | Sliding & Folding | [Andersen Architectural Collection Big Doors product guide](ANDERSEN_BIG) | PANEL DEPTH, FRAME THICKNESS, GLASS |
| Andersen Folding Outswing Door | Andersen | Sliding & Folding | [Andersen Architectural Collection Big Doors product guide](ANDERSEN_BIG) | PANEL THICKNESS, FRAME THICKNESS, GLASS, SILL HEIGHT |
| Sierra Pacific Lift & Slide Door | Sierra Pacific Windows | Sliding & Folding | [Sierra Pacific Doors Catalog, Lift & Slide](https://www.sierrapacificwindows.com/pdf/SPW%20Doors%20Catalog.pdf) | FRAME THICKNESS AND PROJECTION, GLASS |
| LaCantina Aluminum Folding Door (ADA Sill) | LaCantina Doors | Sliding & Folding | [LaCantina 08 35 00 folding door specification (ARCAT)](https://www.arcat.com/sdspecs/htm04/08_35_00lac.htm) |  |
| LaCantina Contemporary Clad Folding Door | LaCantina Doors | Sliding & Folding | [LaCantina 08 35 00 folding door specification (ARCAT)](https://www.arcat.com/sdspecs/htm04/08_35_00lac.htm) | SILL HEIGHT |
| Marvin Ultimate Inswing Door (2-1/4") | Marvin | Entry | [Ultimate Inswing Door Architectural Detail Manual](https://www.marvin.com/f/1019562/x/0484be6dd5/marvin-ultimate-inswing-door-adm.pdf) |  |
| Therma-Tru Classic-Craft Entry Door | Therma-Tru | Entry | [Florida Product Approval FL23957 installation drawings; Therma-Tru Door System Specifications](THERMA_TRU) | STILES AND TOP RAIL; SILL HEIGHT PAIRED FROM THE APPROVAL DRAWING'S LABELS |
| Therma-Tru Entry Door (Public Access Sill) | Therma-Tru | Entry | [Therma-Tru / Endura public access inswing sill](https://learn.reeb.com/knowledge-base/endura-public-access-inswing-sills/) | STILES AND TOP RAIL |
| Pella Fiberglass Entry Door | Pella | Entry | [Pella Fiberglass and Steel Entry Door guide specification](https://media.pella.com/professional/adm/EntryDoor/PellaFiberglass-SteelEntryDoor.rtf) | STILES, RAILS, FRAME THICKNESS, SILL HEIGHT |
| Simpson Wood Entry Door | Simpson Door Company | Entry | [Simpson Door Component Construction data sheet; Florida approval FL39328 sections](https://www.simpsondoor.com/how-they-are-made/Door-Component-Construction-Options.pdf) | RAILS, FRAME (BY OTHERS), THRESHOLD HEIGHT |
| ProVia Signet Fiberglass Entry Door | ProVia | Entry | [ProVia Entry Door Family chart (rev. 3.14.20) and installation guide](https://entrylink.provia.com/doc.aspx?id=309) | RAILS, FRAME THICKNESS, SILL HEIGHT |
| ProVia Embarq Fiberglass Entry Door | ProVia | Entry | [ProVia Entry Door Family chart (rev. 3.14.20)](https://entrylink.provia.com/doc.aspx?id=309) | RAILS, FRAME DEPTH AND THICKNESS, SILL HEIGHT |
| JELD-WEN DF2415 Entry Door System | JELD-WEN | Entry | [JELD-WEN DF2415 Entry Door featurette](https://edge.sitecorecloud.io/jeldwencanada1-jeldwen-p523-e8d9/media/Project/JeldWen/JeldWenCanada/Product-Support/Brochures/PDF/216303-JW-DF2415-Entry-Door-Featurette---12C-008A-JUMP.pdf) | STILES, RAILS, SILL HEIGHT |
| Sierra Pacific Clad Pivot Door | Sierra Pacific Windows | Entry | [Sierra Pacific Pivot Door product data](https://sierrapacificwindows.com/Product/Detail/Door/Pivot/Aluminum%20Clad%20Wood/Pivot%20Door) | FRAME DEPTH (6-9/16" MIN. JAMB + CLAD), FRAME THICKNESS, GLASS |
| FritsJurgens System M+ Pivot Door | FritsJurgens | Interior & Specialty | [FritsJurgens System M+ 40 mm installation manual](https://www.fritsjurgens.com/files/fj.in.m-.40.ind-manual_digital.pdf) |  |
| Steelcraft F-Series Frame - Metal Stud | Steelcraft (Allegion) | Commercial | [Steelcraft Technical Data Manual, Section 2 Frames](https://allegion.ca/content/dam/allegion-us-2/web-files/steelcraft/technical-documents/Section__2__Frames_variations_&_options_and_anchoring_systems_110553.pdf) |  |
| Steelcraft F-Series Frame - Exterior Metal Panel Wall | Steelcraft (Allegion) | Commercial | [Steelcraft Technical Data Manual, Section 2 Frames](https://allegion.ca/content/dam/allegion-us-2/web-files/steelcraft/technical-documents/Section__2__Frames_variations_&_options_and_anchoring_systems_110553.pdf) |  |
| Republic MH Drywall Frame | Republic Doors (Allegion) | Commercial | [Republic MH data sheet TDS 202 / 202a](https://s3.amazonaws.com/s3-absupply-net/pdf/Republic_Doors_Drywall_Frame_Inst_Instr.pdf) |  |
| Ceco SU Frame - CMU | Ceco Door (ASSA ABLOY) | Commercial | [Ceco SU series frame (Sweets)](https://sweets.construction.com/swts_content_files/151593/P250509.htm) |  |
| Kawneer 350 Entrance in Trifab 451T | Kawneer | Commercial | [Kawneer Architectural Detail Manual ADMA010EN (190/350/500 entrances)](KAWNEER) |  |
| Kawneer 190 Narrow Stile Entrance | Kawneer | Commercial | [Kawneer Architectural Detail Manual ADMA010EN](KAWNEER) |  |
| Kawneer 500 Wide Stile Entrance | Kawneer | Commercial | [Kawneer Architectural Detail Manual ADMA010EN](KAWNEER) |  |
| YKK AP 35D Entrance in YES 45 FI | YKK AP | Commercial | [YKK AP Entrances Selection Guide 01-5201-02](https://www.ykkap.com/wp-content/uploads/2026/03/01-5201-02_Entrances.pdf) | TOP AND BOTTOM RAILS (NOT PUBLISHED); THRESHOLD IS THE FT SYSTEM'S |
| YKK AP 50T Thermal Entrance | YKK AP | Commercial | [YKK AP Entrances Selection Guide 01-5201-02](https://www.ykkap.com/wp-content/uploads/2026/03/01-5201-02_Entrances.pdf) | TOP AND BOTTOM RAILS, FRAMING DEPTH |
| Tubelite Medium Stile Entrance | Tubelite (Apogee) | Commercial | [Tubelite Standard and Custom Entrances brochure (2017)](https://tubeliteusa.com/wp-content/uploads/2019/08/Standard-and-Custom-Entrances-2017-web.pdf) | FRAMING (2" x 4-1/2"), THRESHOLD; MEDIUM STILE WIDTH VARIES BY SOURCE |
| Johnson Hardware 1500 Pocket Door | Johnson Hardware | Interior & Specialty | [Johnson Hardware 1500SC installation instructions](https://www.johnsonhardware.com/Content/Images/uploaded/Installation/PD/IN1500SC.pdf) |  |
| Emtek Barn Door - Surface Track | Emtek | Interior & Specialty | [Emtek barn door hardware installation guide](https://www.emtek.com/media/documents/pim-documents/65d/ba029035fae7c5b027.pdf) |  |
| Clopay Sectional Garage Door | Clopay | Interior & Specialty | [Clopay installation manual INST-0130006 (clearance table)](https://literature.clopay.com/pdf_files/INST-0130006_EN.pdf) | SECTION THICKNESS 2" |

## Researched but not drawn (sections not published in a readable form)

- **Behind registration, or a rep only:** Kolbe VistaLuxe and Ultra, Milgard (C650,
  Moving Glass Wall), Fleetwood (3070, 1070), Panoramic Doors, NanaWall (the site blocks
  automated reads), Pella Lifestyle and Architect sliding, Andersen 400 Frenchwood and
  gliding doors, Lualdi, Rimadesio, Linvisibile, Hope's, Portella, Brombal, and Timely (no
  stop or rabbet figures).
- **Image-only drawings:** Therma-Tru ARCH 2, Crittall MW40, and Weather Shield sections.
  Their numbers would have to be read off the drawings.
- **Codes and references used throughout:** 2010 ADA Standards 303 and 404, IRC R311.3 and
  R703.4/R703.8, SDI-111 and SDI-110, ASTM E2112 (via the Building America Solution
  Center), and BIA flashing guidance.
