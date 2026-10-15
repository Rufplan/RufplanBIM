# Window Details — Sources (ADR-123)

These are the Detail Library's window details. Each window has a head, a jamb and a sill.
Hung windows also have a meeting rail section, and sliders an interlock plan. The window
details work like the door details (see docs/DOOR_DETAILS.md): each is redrawn from the
dimensions in the document listed below, not copied from the manufacturer's CAD.

Each detail is inserted as an editable drafting view. A manufacturer detail carries its
source, the URL and a reminder to verify against the manufacturer's current drawings,
printed as text under the drawing. Some sizes are not stated in the source. Those sizes
are drawn at typical values and named under the drawing; the last column below lists
them. Check them before issuing.

The window data is in `crates/studio-core/src/details/doors/window_catalog.rs`. The window
sill and meeting rail are drawn in `doors/windows.rs`. The head and jamb are drawn by the
door engine.

| Window | Maker | Category | Source | Drawn at typical sizes |
|---|---|---|---|---|
| Window - Brick Veneer | Typical | Typical | [IRC R703.8 (lintels, weeps 33" o.c. max.); BIA stepped flashing guide](https://www.gobrick.com/media/file/stepped-flashing.pdf) |  |
| Window - Stucco | Typical | Typical | [ASTM E2112 Method A/B; ASTM C926/C1063 casing beads](https://basc.pnnl.gov/resource-guides/windows-and-doors-are-fully-flashed) |  |
| Window - Double Hung, Lap Siding | Typical | Typical | [ASTM E2112 Method A per Building America Solution Center; IRC R703.4](https://basc.pnnl.gov/resource-guides/windows-and-doors-are-fully-flashed) |  |
| Window - Aluminum in CMU | Typical | Typical | [FGIA (AAMA) 2400/2410 installation; CMU lintel and precast sill practice](https://fgiaonline.org/) |  |
| Window - Aluminum in Steel Stud Rainscreen | Typical | Typical | [FGIA (AAMA) 2400/2410 installation; receptor and subsill practice](https://fgiaonline.org/) |  |
| Window - Wood Picture Window | Typical | Typical | [ASTM E2112 Method A per Building America Solution Center](https://basc.pnnl.gov/resource-guides/windows-and-doors-are-fully-flashed) |  |
| Western Vantage 8670 Casement Window | Western Architectural Openings (Western Window Systems) | Aluminum & Steel | [8670 Casement Window Product Data Sheet (head, jamb, sill, mull sections)](https://d286k0zpa9aa1c.cloudfront.net/asset/495599762709/document_elj1r355hh1v3100po4fego80e?content-disposition=inline) | FRAME / SASH SPLIT OF THE 2-3/8" SIGHTLINE, SASH DEPTH |
| Western Vantage 8660 Awning Window | Western Architectural Openings (Western Window Systems) | Aluminum & Steel | [8660 Awning Window Product Data Sheet](https://www.westernao.com/sites/default/files/2026-06/260501_8660AW_PDS_WAO_R2.pdf) | FRAME / SASH SPLIT OF THE SIGHTLINE, SASH DEPTH |
| Western Performance 7670 Casement Window | Western Architectural Openings (Western Window Systems) | Aluminum & Steel | [7670 Casement Window Product Data Sheet](https://www.westernao.com/sites/default/files/2026-06/document_sois1tr05520n589m94nidu578_0.pdf) | FRAME AND SASH PROFILES (ABOUT 3" FRAME SIZE TO GLASS) |
| Western Performance 7630 Window Wall | Western Architectural Openings (Western Window Systems) | Aluminum & Steel | [7630 Window Wall Product Data Sheet](https://www.westernao.com/sites/default/files/2026-06/document_e348h418054aj2m8acdk7b7b16_0.pdf) |  |
| Marvin Modern Casement Window | Marvin | Aluminum & Steel | [Marvin Modern Casement / Awning Architectural Detail Manual](https://www.marvin.com/f/1019562/x/2ee1e13976/marvin-modern-casement-awning-automated-adm.pdf) | SASH DEPTH; SASH MEMBERS FROM THE 2-11/32" FRAME-TO-GLASS |
| Marvin Modern Picture Window | Marvin | Aluminum & Steel | [Marvin Modern Casement / Awning Architectural Detail Manual](https://www.marvin.com/f/1019562/x/2ee1e13976/marvin-modern-casement-awning-automated-adm.pdf) | FIXED SASH DEPTH |
| Hope's Landmark175 Steel Casement | Hope's Windows | Aluminum & Steel | [Hope's Landmark175 specification sheet (04/2015), Boston Landmarks hearing packet](https://content.boston.gov/sites/default/files/file/2022/07/220721_7lbs_bhac_hearingpresentation.pdf) | FRAME / VENT SPLIT OF THE 2-3/16" SIGHTLINE, VENT DEPTH |
| Optimum RTS430 Thermally Broken Steel Casement | Optimum Window | Aluminum & Steel | [Optimum RTS430 Thermally Broken brochure (glazing, frame, mullions, installation types)](https://www.buildersshow.com/assets/docs/ibs/logos/ExhibitorProductLit_206119_RTS430-Thermally-BrokenBrochure.pdf) | FRAME / VENT SPLIT OF THE 2-1/8" SIGHTLINE, VENT DEPTH |
| Arcadia T200E Window | Arcadia | Aluminum & Steel | [Arcadia T200E Series brochure](https://arcadiainc.com/media/2019/12/Arcadia-Window-T200E-Series-Brochure.pdf) |  |
| Schüco AWS 75.SI+ Tilt-Turn Window | Schüco | Tilt-Turn | [Schüco AWS 75 window system brochure (Thermosash, Schüco partner)](https://www.thermosash.co.nz/media/ev5ndknq/20230418-schueco-aws-75-window-system_-thermosash-schueco-partner-_english.pdf) | FRAME FACE (117 MM LESS THE VENT), VENT DEPTH, 44 MM GLASS; NO PUBLIC INSTALL SECTION |
| Schüco AWS 90.SI+ Tilt-Turn Window | Schüco | Tilt-Turn | [Schüco AWS 90 window system brochure (Thermosash, Schüco partner)](https://www.thermosash.co.nz/media/q23glvud/20230418-schueco-aws-90-window-system_-thermosash-schueco-partner-_english.pdf) | FRAME FACE, VENT DEPTH, 52 MM GLASS; NO PUBLIC INSTALL SECTION |
| Reynaers CS 77 Tilt-Turn Window | Reynaers Aluminium | Tilt-Turn | [Reynaers CS 77 window suite sheet](https://apertureglazing.ie/wp-content/uploads/2022/05/CS-77-window-suite.pdf) | 40 MM GLASS |
| Reynaers MasterLine 8 Tilt-Turn Window | Reynaers Aluminium | Tilt-Turn | [Reynaers MasterLine 8 specification sheet](https://www.barbourproductsearch.info/MasterLine8%20Spec%20Sheet%20A4-file113504.pdf) | 44 MM GLASS |
| Brombal FARO 75 Steel Tilt-Turn Window | Brombal | Tilt-Turn | [Brombal FARO 75 section sheet](https://discoverbrombal.com/wp-content/uploads/2025/03/BROMBAL-FARO-75.pdf) | SASH DEPTH, 40 MM GLASS, WHICH SIGHTLINE IS FRAME OR VENT |
| Marvin Ultimate Double Hung G2 | Marvin | Double Hung & Sliding | [Ultimate Double Hung G2 Architectural Detail Manual](https://www.marvin.com/f/1019562/x/ce0052b4df/adm-ultimate-double-hung-g2.pdf) | MEETING RAIL READ AS 1-7/16"; R.O. ABOUT FRAME + 1/2" |
| Marvin Ultimate Glider | Marvin | Double Hung & Sliding | [Ultimate Glider Architectural Detail Manual](https://marvin.com/f/1019562/x/8ddfa55a1d/adm-ultimate-glider.pdf) | SASH RAILS |
| Marvin Elevate Double Hung | Marvin | Double Hung & Sliding | [Elevate Double Hung Architectural Detail Manual](https://www.marvin.com/f/1019562/x/449f2a8346/adm-elevate-double-hung.pdf) | SASH MEMBERS (FROM THE 2-7/8" HEAD AND 2-3/8" CHECK-RAIL LABELS) |
| Marvin Essential Double Hung | Marvin | Double Hung & Sliding | [Essential Double Hung Architectural Detail Manual](https://marvin.com/f/1019562/x/b588a59fb0/marvin-essential-double-hung-adm.pdf) | SASH MEMBERS, GLASS THICKNESS |
| Andersen 400 Series Woodwright Double-Hung | Andersen | Double Hung & Sliding | [400 Series Product Guide, Woodwright double-hung details](https://www.kuikenbrothers.com/wp-content/uploads/400-series-product-guide.pdf) | FRAME THICKNESS, SASH MEMBERS (ABOUT 3" FRAME + STILE PER SIDE) |
| Andersen A-Series Double-Hung | Andersen | Double Hung & Sliding | [A-Series Product Guide, double-hung details](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/2020-to-current-literature/a-series-product-guide.pdf) | FRAME THICKNESS, SASH MEMBERS (ABOUT 3-5/16" PER SIDE) |
| Andersen 400 Series Gliding Window | Andersen | Double Hung & Sliding | [400 Series Product Guide, gliding window details](https://www.kuikenbrothers.com/wp-content/uploads/400-series-product-guide.pdf) | FRAME THICKNESS, SASH MEMBERS |
| Sierra Pacific Monument Clad Double Hung | Sierra Pacific Windows | Double Hung & Sliding | [Sierra Pacific double hung detail sheets (head, meeting rail, sill, jamb at 4"=1'), Detroit HDC submittal](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2026-08/HDC2026-00439_Cycle%202.pdf) | FIN-TO-FACE 1-1/8", SASH MEMBERS, R.O. ABOUT UNIT + 1/2" |
| Sierra Pacific All-Wood Double Hung | Sierra Pacific Windows | Double Hung & Sliding | [Sierra Pacific Wood DH head, sill and jamb details, Detroit HDC submittal](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2026-01/Sierra%20Pacific_Wood%20DH%20Arch%20OP%20He.pdf) | FRAME THICKNESS, SASH MEMBERS |
| Kolbe Heritage Sterling Double Hung | Kolbe Windows & Doors | Double Hung & Sliding | [Kolbe Heritage Series brochure (spec text), Detroit HDC submittal](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2026-03/2008%20LONGFELLOW%20-%20HDC%20-%20SPECS%20-%20REVISED%20260311.pdf) | SASH MEMBERS; NO SECTIONS PUBLISHED, SPEC TEXT ONLY |
| Loewen Double Hung | Loewen | Double Hung & Sliding | [Loewen double / single hung technical details (head, sill, plan)](https://technical.loewen.com/double-single-hung-windows/) | SASH THICKNESS, GLASS; STILES AND RAILS FROM THE SIGHTLINES LESS THE FRAME |
| Marvin Ultimate Casement | Marvin | Casement & Awning | [Ultimate Casement / Awning / Picture Architectural Detail Manual](https://marvin.com/f/1019562/x/eaabd5bb0d/adm-ultimate-casement-awning-picture.pdf) | SASH STILES AND RAILS DERIVED FROM THE SIZE TABLES (ABOUT 1-1/2") |
| Marvin Ultimate Awning | Marvin | Casement & Awning | [Ultimate Casement / Awning / Picture Architectural Detail Manual](https://marvin.com/f/1019562/x/eaabd5bb0d/adm-ultimate-casement-awning-picture.pdf) | SASH STILES AND RAILS DERIVED FROM THE SIZE TABLES |
| Marvin Ultimate Casement Picture | Marvin | Casement & Awning | [Ultimate Casement / Awning / Picture Architectural Detail Manual](https://marvin.com/f/1019562/x/eaabd5bb0d/adm-ultimate-casement-awning-picture.pdf) | FIXED SASH MEMBERS FROM THE 2-3/32" SIGHTLINE LESS THE FRAME |
| Marvin Elevate Casement | Marvin | Casement & Awning | [Elevate Casement / Awning Architectural Detail Manual (2026-04-21)](https://marvin.com/f/1019562/x/8e5679005e/adm-elevate-casement-awning.pdf) | SASH MEMBERS FROM THE 2-19/32" SIGHTLINE LESS THE FRAME |
| Andersen 400 Series Casement | Andersen | Casement & Awning | [400 Series Product Guide, casement / awning / picture details](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/1903_400series_pg_lr-compressed-22.pdf) | FRAME / SASH SPLIT OF THE SIGHTLINE, SASH THICKNESS, GLASS |
| Andersen A-Series Casement | Andersen | Casement & Awning | [A-Series Product Guide, casement details](https://edge.sitecorecloud.io/andersencor0d3d-andersencorae0f-prod102c-dc69/media/Project/AndersenCorporation/AndersenWindows/AndersenWindows/files/brochures/2020-to-current-literature/a-series-product-guide.pdf) | FRAME / SASH SPLIT OF THE SIGHTLINES, SASH THICKNESS, GLASS |
| Pella Reserve Traditional Casement | Pella | Casement & Awning | [Pella Reserve Traditional Casement ADM (2026), vent head / sill / jambs](https://media.pella.com/professional/adm/Clad-Wood/Pella-Reserve_Casement.pdf) | FRAME THICKNESS (THE 1-7/8" LABEL READ AS THE STILE) |
| Pella Reserve Contemporary Casement | Pella | Casement & Awning | [Pella Reserve Contemporary Casement ADM](https://media.pella.com/professional/adm/Clad-Wood/Pella-ReserveCont_Casement.pdf) | FRAME THICKNESS |
| Pella Lifestyle Series Casement | Pella | Casement & Awning | [Pella Lifestyle Series Casement ADM (2026), unit sections](https://media.pella.com/professional/adm/Clad-Wood-LS/Pella-LifestyleSeries-Casement.pdf) | FRAME / SASH SPLIT OF THE 2-7/16" SIGHTLINE |
| Sierra Pacific Clad Casement | Sierra Pacific Windows | Casement & Awning | [Sierra Pacific Clad Casement Architectural Design Manual](https://www.sierrapacificwindows.com/Technical/Window/Casement/Aluminum%20Clad%20Wood/Casement/Architectural%20Design%20Manual/Clad%20Casement%20ADM.pdf) |  |
| Loewen Casement | Loewen | Casement & Awning | [Loewen casement technical details (head, sill, plan)](https://technical.loewen.com/casement-windows/) | FRAME THICKNESS, SASH MEMBERS, GLASS (1/2" AIRSPACE STATED) |
| Weather Shield Signature Casement | Weather Shield | Casement & Awning | [Weather Shield Signature 8219 casement Quick Spec and sections](https://weathershield.com/wp-content/uploads/2022/01/SG_8219_CA_QS.pdf) | FRAME, SASH AND GLASS (SECTION PDFS ARE OUTLINED, NOT READABLE) |
| Windsor Pinnacle Casement | Windsor Windows & Doors | Casement & Awning | [Windsor Pinnacle Casement & Awning sell sheet](https://www.windsorwindows.com/userdocs/documents/Pinnacle_Casement_and_Awning_Sell_Sheet.pdf) | FRAME DEPTH, SASH STILES AND RAILS (NO SECTIONS FOUND) |
| Milgard Tuscany Single Hung | Milgard | Double Hung & Sliding | [Milgard Tuscany single hung CAD detail (1.375" setback)](https://www.milgard.com/sites/default/files/new-series/tuscany-series/tuscany-series-single-hung-window/field_series_detail_cad_files/Tuscany_SH_1.375_setback.pdf) | SASH THICKNESS, GLASS (ABOUT 3/4"), SASH MEMBERS FROM THE SIGHTLINES |
| Milgard Tuscany Horizontal Slider | Milgard | Double Hung & Sliding | [Milgard Tuscany 8120T slider CAD detail (1.375" setback)](https://www.milgard.com/sites/default/files/2023-09/Tuscany_8120T_HVB_1.375in.pdf) | SASH THICKNESS, GLASS, SASH MEMBERS FROM THE SIGHTLINES |
| Milgard C700 Fiberglass Single Hung | Milgard | Double Hung & Sliding | [Milgard C700 12210 single hung CAD details](https://www.milgard.com/sites/default/files/2026-01/C700_12210_SH_BLOCK_FRAME.pdf) | SASH THICKNESS, GLASS |
| Andersen 100 Series Single-Hung | Andersen | Double Hung & Sliding | [Andersen 100 Series Product Guide for Professionals](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2024-10/product-guide-100-series-window-door-9045491.pdf) | SASH THICKNESS, GLASS, FRAME / SASH SPLIT OF THE 3-1/8" SIGHTLINE |
| Andersen 100 Series Gliding Window | Andersen | Double Hung & Sliding | [Andersen 100 Series Product Guide for Professionals](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2024-10/product-guide-100-series-window-door-9045491.pdf) | SASH THICKNESS, GLASS |
| Simonton Reflections 5500 Double Hung | Simonton | Double Hung & Sliding | [Simonton Reflections 5500 double hung CAD drawing](https://marketing.simonton.com/download/reflections-5500-double-hung-with-nail-fin-and-j-channel-cad-drawing/) | FIN SETBACK ABOUT 7/8", SASH THICKNESS, GLASS, SASH MEMBERS |
| Pella 250 Series Double-Hung | Pella | Double Hung & Sliding | [Pella 250 Series Double-Hung ADM](https://media.pella.com/professional/adm/Vinyl250/Pella-250Series_DoubleHung.pdf) | SASH THICKNESS AND MEMBERS (SECTION LABELS UNASSIGNED) |
| Pella Impervia Single-Hung | Pella | Double Hung & Sliding | [Pella Impervia Single-Hung ADM](https://media.pella.com/professional/adm/Fiberglass/Pella-Impervia_SingleHung.pdf) | SASH THICKNESS AND MEMBERS; CHECK RAIL READ AS 2-3/8" |
| Pella Reserve Traditional Double-Hung | Pella | Double Hung & Sliding | [Pella Reserve Double-Hung ADM (Detroit HDC copy)](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2026-02/LONGFELLOW%20-%20Pella-Reserve_DoubleHung.pdf) | FRAME THICKNESS, RAILS, MEETING RAIL; R.O. NOT FOUND |
| Pella Lifestyle Series Double-Hung | Pella | Double Hung & Sliding | [Pella Lifestyle Double-Hung ADM, LS-DH-24 to 27 (Detroit HDC copy)](https://detroitmi.gov/sites/detroitmi.localhost/files/hdc-submitted-materials/2026-08/Pella%20ADM%20Double%20Hung.pdf) | FRAME THICKNESS, STILES AND RAILS |
| JELD-WEN Siteline Double Hung | JELD-WEN | Double Hung & Sliding | [JELD-WEN Siteline double hung architectural detail manual (2024), San Antonio OHP file](https://gis.sanantonio.gov/OHPSystemDocs/api/Attachments/GetRequestAttachment/%7BF0704D94-0000-C215-8C64-E4718BA41D65%7D) | SASH THICKNESS, GLASS, STILES AND RAILS |
| Window - Egress Casement | Typical | Typical | [IRC R310 emergency escape and rescue openings (2021 R310.1-R310.2)](https://bouldercounty.gov/property-and-land/land-use/building/building-publications/b10-emergency-escape-and-rescue-openings/) |  |
| Kawneer 8225TL Thermal Fixed Window | Kawneer | Commercial | [Kawneer 8225TL Architectural Detail Manual ADME010EN (receptors, sub-sills, anchors)](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADME010EN.pdf) |  |
| Kawneer AA 4325 Window | Kawneer | Commercial | [Kawneer AA 4325 Architectural Detail Manual ADME060EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADME060EN.pdf) |  |
| Kawneer 8400TL Thermal Window | Kawneer | Commercial | [Kawneer 8400TL Architectural Detail Manual ADME080EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADME080EN.pdf) |  |
| Kawneer Trifab 451T Punched Window | Kawneer | Commercial | [Kawneer Trifab VersaGlaze 451/451T Architectural Detail Manual ADMC040EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMC040EN.pdf) |  |
| YKK AP YOW 350 T Window | YKK AP | Commercial | [YKK AP YOW 350 T data sheet 05-5007-10; YOW 350 TUH design manual](https://www.ykkap.com/commercial/productguide/product/05-5007-10.pdf) |  |
| YKK AP YES 45 TU Punched Window | YKK AP | Commercial | [YKK AP YES 45 TU / FI data sheets 02-5003-15 / 02-5003-16](https://www.ykkap.com/commercial/productguide/product/02-5003-16.pdf) | SILL FLASHING DETAILS (FROM SUMMARIES; THE INSTALLATION PDF WOULD NOT OPEN) |
| Tubelite T14000 Punched Window | Tubelite (Apogee) | Commercial | [Tubelite T14000 product drawings PS_T14000 (Sept. 2024) and installation manual](https://media.tubeliteusa.com/wp-content/uploads/2024/09/10112236/PS_T14000_Sept.-2024.pdf) |  |
| Oldcastle FG-3000 Punched Window | Oldcastle BuildingEnvelope | Commercial | [Oldcastle BuildingEnvelope architectural glazing systems catalog; Series 3000 detail sheet](https://www.syracuseglass.com/E-DOCS/Architectural%20Aluminum/EDOCS/Oldcastle%20BuildingEnvelope%20Architectural%20Glazing%20Systems.pdf) | SUBSILL PROFILE (THE DETAIL PDF WOULD NOT EXTRACT) |
| Graham GT6700 Window | Graham Architectural Products | Commercial | [Graham GT6700 data sheet](https://www.grahamwindows.com/wp-content/uploads/GT6700_Data.pdf) | RECEPTOR AND SUBSILL |
| EFCO 325X Window | EFCO | Commercial | [EFCO 325X brochure](https://sweets.construction.com/swts_content_files/4/710505.pdf) | SIGHTLINE, RECEPTOR AND SUBSILL |
| Winco 3250 Window | Winco Window | Commercial | [Winco 3250 Series book and receptor sheet](https://www.wincowindow.com/sites/default/files/2026-06/3250_receptors.pdf) | SIGHTLINE |

## Researched but not drawn

- **Section drawings behind registration or a sales rep:** Kolbe (Ultra, VistaLuxe),
  Lincoln, Fleetwood, Zola, Intus, Euroline, Portella, Hope's Jamestown175, Wausau and
  TRACO.
- **Drawings published as images only, with no readable dimensions:** Andersen E-Series,
  Weather Shield sections, Crittall W20, Ply Gem and Anlin.
- **Codes and references used:**
  - ASTM E2112: flashing widths, pan rear-leg heights, shims at 12" o.c. maximum.
  - BIA Technical Note 28: 1" air space, rowlock sill sloped 15 degrees, end dams.
  - IRC R703.8: lintels and weeps.
  - IRC R310: egress window openings.
  - ASTM C1063: stucco casing beads.
  - NCMA TEK 19-05A: flashing in CMU walls.
  - PNNL Building America Solution Center: window detail sequences.
