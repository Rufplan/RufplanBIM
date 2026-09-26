# Material previews

Path-traced cube previews of the material library (ADR-029), 256 × 256 WebP. They were
rendered with the app's own preview renderer (`src/render/preview.ts`):

- 384 samples per pixel;
- the studio lighting in `../backgrounds/studio.hdr` (Poly Haven's `studio_small_09`, CC0);
- 1K versions of each material's textures.

Photo textures come from [Poly Haven](https://polyhaven.com/textures), released under CC0.
The app downloads their 2K maps on first use and caches them on the computer. The
procedural textures (tile, CMU, carpet, marble veining, brushed metal) are generated in
`src/render/materials.ts`.

To re-render after changing presets, run the thumbnail tool described in ADR-029.
