# Handoff: Rufplan Studio App Icon (option 2a — "Blueprint R, dark")

## Overview
This is the application icon for the **Rufplan Studio** desktop app (`rufplan-studio.exe`). It replaces the current placeholder "RS" icon. The icon uses the Rufplan web brand: an ink tile, heavy condensed type, the cyan accent, and the bar over the Ū in the RŪFPLAN wordmark.

## About the Design Files
The icon was designed in HTML/SVG as a design reference. The PNG and ICO files in this bundle were rendered from that design and can be used directly. The SVG is the source master, but its **R is live text**. Before shipping the SVG, or before using it as a vector source in a build pipeline, convert the R to an outline in Figma or Illustrator.

## Fidelity
**High-fidelity.** The colours, geometry and type below are final.

## Geometry (256 × 256 artboard; all other sizes scale proportionally)
| Layer | Spec |
|---|---|
| Tile | 256×256, corner radius **28**, fill `#0B0F14` |
| Grid | Lines at x = 64, 128, 192 and y = 64, 128, 192; stroke `#243441`, width **3**; clipped to the tile |
| Macron bar | Rect x **78**, y **30**, w **100**, h **18**, fill `#1FA3D8`, square corners |
| Letter "R" | Barlow Condensed **ExtraBold (800)**, size **232**, centred at x **128**, baseline y **222**, fill `#FFFFFF` |

The macron is centred over the R, as in the RŪFPLAN wordmark. The R is aligned to the grid, with its stem close to the x = 64 grid line.

## Design Tokens
- Ink (tile): `#0B0F14`
- Grid line: `#243441`
- Cyan accent: `#1FA3D8`
- White: `#FFFFFF`
- Font: Barlow Condensed 800 (Google Fonts, OFL licence)

## Small sizes
- At 16px and 24px the grid is only faintly visible. That is intended: the icon reads as the R plus the cyan bar.
- If you hand-hint the 16px version, round the macron to exactly 1px tall on whole-pixel rows, and snap the R stem to the pixel grid.

## Platform integration
- **Windows:** use `rufplan-studio.ico` (it contains 16, 24, 32, 48, 64, 128 and 256px sizes, stored as PNGs).
  - Electron: set `build.win.icon` in the electron-builder config.
  - Native: use a `.rc` resource.
- **macOS:** build an `.icns` from `png/` using `iconutil`. Put the 1024 file in `icon_512x512@2x`.
  - Note: macOS Big Sur and later expects roughly 10% transparent padding around the tile. If you target Mac, scale the tile to about 824/1024 and centre it.
- **Web/favicon:** use `png/rufplan-studio-32.png`, plus the 180px size (resampled from the 256) for the Apple touch icon.

## Files
- `rufplan-studio-icon.svg`: vector master (outline the R before shipping)
- `rufplan-studio.ico`: Windows multi-size icon
- `png/rufplan-studio-{16,24,32,48,64,128,256,512,1024}.png`: rendered PNGs with a transparent background outside the rounded corners
- `preview.html`: shows every size side by side
- `Rufplan Studio Icon.dc.html`: the full exploration board. The chosen icon is card **2a**.
