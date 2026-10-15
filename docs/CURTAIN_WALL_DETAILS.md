# Curtain Wall Details — Sources (ADR-125)

These are the Detail Library's curtain wall details. Each system has a head, sill and
jamb, plus a vertical mullion, a horizontal, and a slab edge showing the spandrel, back
pan, fire safing, smoke seal and anchor. Unitized systems also have a stack joint.

Every detail is redrawn from the face width, system depth, glass and the notes in the
document listed for its system, not copied from the manufacturer's CAD. Each one inserts
as an editable drafting view. Under the drawing it carries its source, the source's URL,
and a reminder to check it against the manufacturer's current details, engineering and
shop drawings.

Some sizes are not stated in the source. These are drawn at typical values and listed in
the last column below, and also under each detail. Cover cap depth and glass bite are the
usual ones; most manufacturers publish them only in their CAD files.

The system data is in `crates/studio-core/src/details/doors/curtain_catalog.rs`. The
drawing code is in `doors/curtain.rs`.

| System | Maker | Category | Source | Drawn at typical sizes |
|---|---|---|---|---|
| Curtain Wall - Captured Stick System | Typical | Typical | [WBDG Building Envelope Design Guide, Curtain Walls; AAMA CW-DG-1](https://www.wbdg.org/guides-specifications/building-envelope-design-guide/fenestration-systems/curtain-walls) |  |
| Curtain Wall - Four-Sided Structural Silicone | Typical | Typical | [WBDG Building Envelope Design Guide, Curtain Walls; ASTM C1401 structural sealant glazing](https://www.wbdg.org/guides-specifications/building-envelope-design-guide/fenestration-systems/curtain-walls) |  |
| Curtain Wall - Unitized System | Typical | Typical | [WBDG Building Envelope Design Guide, Curtain Walls (unitized systems)](https://www.wbdg.org/guides-specifications/building-envelope-design-guide/fenestration-systems/curtain-walls) |  |
| Curtain Wall - At Masonry Opening | Typical | Typical | [WBDG Building Envelope Design Guide, Curtain Walls; ASTM C1193 sealant joints](https://www.wbdg.org/guides-specifications/building-envelope-design-guide/fenestration-systems/curtain-walls) |  |
| Curtain Wall - Perimeter Fire Containment | Typical | Typical | [Thermafiber Technical Bulletin CW-01 (ASTM E2307 perimeter fire containment); IBC 715.4](https://www.owenscorning.com/dms/10020920) |  |
| Curtain Wall - Perimeter Flashing & Air Barrier | Typical | Typical | [GCP Applied Technologies curtain wall head / jamb / sill details NPL-020 to NPL-022; WBDG Curtain Walls](https://gcpat.com/sites/gcpat.com/files/2020-11/NPL-022_CURTAINWALL_SILL_DETAIL_NON-MASONRY_CLADDING.pdf) |  |
| Kawneer 1600 Wall System 1 | Kawneer | Captured | [Kawneer 1600 Wall System 1 Architectural Detail Manual ADMD010EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD010EN.pdf) | COVER CAP DEPTH, GLASS BITE (CAD ONLY) |
| Kawneer 1600UT System 1 | Kawneer | Captured | [Kawneer 1600UT System 1 Architectural Detail Manual ADMD080EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD080EN.pdf) | COVER CAP DEPTH |
| Kawneer 1600UT System 1 (Triple Glazed) | Kawneer | Captured | [Kawneer 1600UT System 1 Architectural Detail Manual ADMD080EN (1-3/4" details)](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD080EN.pdf) | COVER CAP DEPTH |
| Kawneer 1620 Curtain Wall | Kawneer | Captured | [Kawneer 1620 Architectural Detail Manual ADMD210EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD210EN.pdf) | COVER CAP DEPTH |
| Kawneer 1620UT Curtain Wall | Kawneer | Captured | [Kawneer 1620UT Architectural Detail Manual ADMD215EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD215EN.pdf) | COVER CAP DEPTH |
| Kawneer 1600 SS Curtain Wall | Kawneer | Captured | [Kawneer 1600 SS Architectural Detail Manual ADMD060EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD060EN.pdf) | COVER CAP DEPTH |
| YKK AP YCW 750 OG Curtain Wall | YKK AP | Captured | [YKK AP YCW 750 OG data sheet 04-5005-11 and installation manual 04-4005-18](https://www.ykkap.com/commercial/productguide/instman/04-4005-18.pdf) | COVER CAP DEPTH |
| YKK AP YCW 750 XT Curtain Wall | YKK AP | Captured | [YKK AP YCW 750 XT data sheet 04-5016-08](https://www.ykkap.com/commercial/productguide/product/04-5016-08.pdf) | COVER CAP DEPTH |
| YKK AP YHC 300 OG Impact Curtain Wall | YKK AP | Captured | [YKK AP YHC 300 OG data sheet 04-5014-06](https://www.ykkap.com/commercial/productguide/product/04-5014-06.pdf) | COVER CAP DEPTH |
| EFCO 5600 Curtain Wall | EFCO (Apogee) | Captured | [EFCO 5600 product data and XTherm 5600 installation instructions](https://apogeearchmetals.com/products/5600-2-1-2-curtain-wall/) | COVER CAP DEPTH; 7-1/4" DEPTH CHOSEN FROM THE RANGE |
| EFCO XTherm 5600 Curtain Wall | EFCO (Apogee) | Captured | [EFCO XTherm 5600 installation instructions](https://apogeearchmetals.com/wp-content/uploads/EFCO_560022525Thermal_Installation-Instructions-1.pdf) | SYSTEM DEPTH, COVER CAP DEPTH |
| Tubelite 400T Curtain Wall | Tubelite (Apogee) | Captured | [Tubelite 400T Curtain Wall Design Manual, 7.25" depth thermal](https://apogeearchmetals.com/wp-content/uploads/Tubelite_400T-Curtain-Wall_Design-Manual_7.25in-Depth_Thermal.pdf) |  |
| Tubelite 400CW Curtain Wall | Tubelite (Apogee) | Captured | [Tubelite 400CW Curtain Wall Design Manual, 7.5" depth thermal](https://apogeearchmetals.com/wp-content/uploads/Tubelite_400CW-Curtain-Wall_Design-Manual_7.5in-Depth_Thermal.pdf) |  |
| Tubelite 400TU Curtain Wall | Tubelite (Apogee) | Captured | [Tubelite 400TU Curtain Wall Design Manual](https://apogeearchmetals.com/wp-content/uploads/Tubelite_400TU-Curtain-Wall_Design-Manual.pdf) | COVER CAP DEPTH |
| Wausau HP-Wall Curtain Wall | Wausau Window and Wall (Apogee) | Captured | [Wausau HP-Wall product data](https://apogeearchmetals.com/products/hp-walltm-curtain-wall/) | COVER CAP DEPTH |
| Arcadia T500 OPG-1900 Curtain Wall | Arcadia | Captured | [Arcadia Thermal OPG1900 detail set and T500 guide specification](https://arcadiainc.com/media/2026/06/Framing-Arcadia-Thermal-OPG1900-ALL-Series.pdf) | COVER CAP DEPTH |
| Oldcastle Reliance Curtain Wall | Oldcastle BuildingEnvelope | Captured | [Oldcastle BuildingEnvelope Reliance installation manual](https://obe.com/systemDetails?categoryCode=reliance) | COVER CAP DEPTH |
| Oldcastle Reliance-TC Curtain Wall | Oldcastle BuildingEnvelope | Captured | [Oldcastle BuildingEnvelope Reliance-TC product data](https://obe.com/systemDetails?categoryCode=reliance_tc) | COVER CAP DEPTH |
| CRL-U.S. Aluminum 3250 Curtain Wall | CRL-U.S. Aluminum | Captured | [U.S. Aluminum Series 3250 architectural manual section J3](https://crlaurence.com.au/techdocs/usalum/adms/curtain-wall-3250.pdf) | COVER CAP DEPTH |
| Kawneer 1600 Wall System 2 (SSG) | Kawneer | Structural Silicone | [Kawneer 1600 Wall System 2 Architectural Detail Manual ADMD020EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD020EN.pdf) | COVER CAP DEPTH AT HORIZONTALS |
| Kawneer 1620 SSG Curtain Wall | Kawneer | Structural Silicone | [Kawneer 1620 SSG Architectural Detail Manual ADMD211EN](https://www.kawneer.com/kawneer_files/shared%20files/97911-Arch_Manual/ADMD211EN.pdf) | COVER CAP DEPTH AT HORIZONTALS |
| YKK AP YCW 750 SSG (Four-Sided) | YKK AP | Structural Silicone | [YKK AP YCW 750 SSG data sheet 04-5030-07](https://www.ykkap.com/commercial/productguide/product/04-5030-07.pdf) |  |
| YKK AP YCW 750 XT SSG | YKK AP | Structural Silicone | [YKK AP YCW 750 XT SSG data sheet 04-5027-03](https://www.ykkap.com/commercial/productguide/product/04-5027-03.pdf) | COVER CAP DEPTH AT HORIZONTALS |
| Kawneer 2500 UT Unitwall | Kawneer | Unitized | [Kawneer 2500 UT Unitwall Architectural Detail Manual ADMD230EN](https://www.kawneer.us/kawneer_files/shared%20files/97911-Arch_Manual/ADMD230EN.pdf) | CAP DEPTH, SPLIT-MULLION HALVES |
| Kawneer 2500 PG Unitwall | Kawneer | Unitized | [Kawneer 2500 PG Architectural Detail Manual ADMD120EN](https://kawneer.us/kawneer_files/shared%20files/97911-Arch_Manual/ADMD120EN.pdf) | CAP DEPTH, THERMAL BREAK (NOT STATED) |
| YKK AP YUW 750 XT Unitized Wall | YKK AP | Unitized | [YKK AP YUW 750 XT data sheet 04-5020-08 and installation manuals](https://www.ykkap.com/commercial/productguide/product/04-5020-08.pdf) | CAP DEPTH, SPLIT-MULLION HALVES |
| Schüco TropTec UC 65 SG Unitized | Schüco | Unitized | [Schüco TropTec UC 65 SG.NI system information](https://www.schueco.com/resource/blob/5532534/24dc9aba9b72cba86e8795d95b53d77d/_TropTec_UC_65_SG_NI_GSI__.pdf?domain=in) | CAP / SG JOINT PROFILE |
| Reynaers CW 86-EF Element Façade | Reynaers Aluminium | Unitized | [Reynaers CW 86 brochure](https://reynaers.com/resource/49916/download) | 32 MM GLASS, STACK JOINT MOVEMENT (NOT PUBLISHED) |
| WICONA WICTEC 50EL Unitized | WICONA | Unitized | [WICONA WICTEC 50EL / EL60 data sheets (02.2020)](https://integratedfs.com/wp-content/uploads/2023/08/WICONA-WICTEC-EL60-Unitized-curtain-wall.pdf) | 36 MM GLASS, CAP, STACK JOINT MOVEMENT |
| Schüco FWS 50 Mullion-Transom Façade | Schüco | European | [Schüco FWS 50 product data (baukobox catalogue)](https://baukobox.de/en/produkte/690-schueco-international-kg-schueco-facade-fw-50) | MULLION DEPTH 150 MM + CAP, 32 MM GLASS |
| Schüco FWS 60 Mullion-Transom Façade | Schüco | European | [Schüco FWS 60 product data (baukobox catalogue)](https://baukobox.de/en/products/2904-schueco-international-kg-schueco-facade-fws-60) | MULLION DEPTH 150 MM + CAP, 32 MM GLASS |
| Reynaers CW 50 Curtain Wall | Reynaers Aluminium | European | [Reynaers CW 50 datasheet (09/2023)](https://reynaers.pl/resource/50499/download) | MULLION DEPTH 150 MM + CAP, 32 MM GLASS |
| Reynaers CW 60 Curtain Wall | Reynaers Aluminium | European | [Reynaers CW 60 datasheet](https://reynaers.com/resource/49804/download) | MULLION DEPTH, 32 MM GLASS |
| WICONA WICTEC 50 Stick Curtain Wall | WICONA | European | [WICONA WICTEC 50 data sheet](https://integratedfs.com/wp-content/uploads/2023/08/WICONA-WICTEC-50-stick-curtain-system.pdf.pdf) | PROFILE DEPTH, 32 MM GLASS |
| Aluprof MB-SR50N Curtain Wall | Aluprof | European | [Aluprof MB-SR50N technical sheet (EN-07.2017)](https://aluprof.eu/files/downloads/mb-sr50n_a3_en.pdf) | MULLION DEPTH, 32 MM GLASS |
| RAICO THERM+ A-I Curtain Wall | RAICO | European | [RAICO THERM+ A-I product data (ArchDaily catalogue)](https://www.archdaily.com/catalog/us/products/34639/aluminium-facades-glass-roofs-therm-a-i-raico) | PROFILE DEPTH, 32 MM GLASS |

## References used for the typical details

- **Fire containment:** IBC 715.4 and ASTM E2307 perimeter fire containment, from
  Thermafiber Technical Bulletin CW-01 and the ROCKWOOL Perimeter Fire Containment
  Technical Guide.
  - Spandrel insulation at least 2" thick, mechanically attached.
  - A T-bar stiffener at least 20 ga at the floor line.
  - Safing at least 4" deep, compressed 33%.
  - Smoke sealant with a 1" overspray.
  - Back pans at least 20 ga galvanized steel.
- **Perimeter:** the WBDG Curtain Walls guide, and the GCP Applied Technologies head,
  jamb and sill details NPL-020 to NPL-022.
  - Sill flashing with end dams.
  - Self-adhered flashing lapped at least 3".
  - An interior air seal over backer rod.
  - A transition membrane to the wall's air barrier.
- **Installation:** YKK AP installation manuals.
  - 1/2" perimeter joints.
  - Pressure plates screwed at 9" o.c.
  - Setting blocks at the 1/4 points.
  - Weeps 5/16".

## Researched but not drawn

- **Systems no longer offered:** Wausau INvision unitized, EFCO 5900 and 7000, and EFCO
  6500 and 8000 unitized.
- **Data not public:** Schüco UCC 65 SG.
- **No data found:** Tubelite INvision.
- **Section drawings published only as vector graphics with no readable dimensions:**
  the EFCO and Wausau design manuals.
