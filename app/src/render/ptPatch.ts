// Light transport improvements to three-gpu-pathtracer 0.0.24 (ADR-118), made to the tracing
// material's shader text when a render job starts, so no fork of the library is needed:
//
// 1. The sky (which carries our sun) gets a fixed share of the light samples (`envPick`)
//    instead of 1 / (lights + 1): with thirty fixtures on, the sun's shadows had 3% of the
//    samples and daylit interiors filled with speckle.
// 2. Indirect light is clamped (`clampIndirect`, as Cycles' Clamp Indirect, Corona's Max
//    Sample Intensity and V-Ray's Max Ray Intensity): a diffuse bounce that happens to find
//    the sun or a lamp lens can't put a firefly in the image. Light seen directly is never
//    clamped.
// 3. Diffuse transmission for leaves and grass blades (as V-Ray's two-sided material and
//    Corona's translucency): a material with `castShadow = 2` scatters the share
//    `transmission` of the light it receives out of its back, cosine-weighted and tinted by
//    its sheen colour (a leaf glows yellow-green with the sun behind it), receives the sun
//    and sky through its back, and is opaque to shadow rays. The library's thin
//    transmission instead let leaves be partly see-through.
//
// Each change is a string replacement that must match exactly; if any doesn't (a library
// update), none is applied and the stock tracer renders.
import type * as THREE from "three";

/** One replacement: what to find, how many times it must occur, what to put. */
type Edit = [find: string, count: number, replace: string];

const EDITS: Edit[] = [
  // Globals and uniforms.
  [
    "uniform int seed;",
    1,
    "uniform int seed;\nuniform float envPick;\nuniform float clampIndirect;",
  ],
  [
    "float lightsDenom;",
    1,
    `float lightsDenom;
				float lightSel;
				float envSel;
				vec3 dtFront;
				vec3 dtBack;`,
  ],
  [
    "void main() {",
    1,
    `vec3 clampMax( vec3 c, float m ) {
					float l = max( c.r, max( c.g, c.b ) );
					return l > m ? c * ( m / l ) : c;
				}

				void main() {`,
  ],
  // 1. The selection probabilities, set once the environment is known.
  [
    "\t\t\t\t\t// final color\n",
    1,
    `					bool envOn = environmentIntensity != 0.0 && envMapInfo.totalSum != 0.0;
					envSel = lights.count == 0u ? 1.0 : ( envOn ? envPick : 0.0 );
					lightSel = lights.count == 0u ? 0.0 : 1.0 - envSel;

					// final color
`,
  ],
  [
    "lightsDenom != 0.0 && rand( 5 ) < float( lights.count ) / lightsDenom",
    1,
    "lights.count != 0u && rand( 5 ) >= envSel",
  ],
  // Each light chosen by its power over its distance from the point lit (ADR-118), so a
  // room's own lamps get the samples, not the thirty others in the house; the same
  // probability is recomputed where a bounce hits an area light (MIS).
  [
    "LightRecord randomLightSample( sampler2D lights, sampler2DArray iesProfiles, uint lightCount, vec3 rayOrigin, vec3 ruv ) {",
    1,
    `float lightWeight( Light l, vec3 p ) {

		if ( l.type == DIR_LIGHT_TYPE ) return 1e30;
		vec3 d = l.position - p;
		float k = ( l.type == RECT_AREA_LIGHT_TYPE || l.type == CIRC_AREA_LIGHT_TYPE ) ? l.area : 1.0;
		return max( luminance( l.color ) * l.intensity * k / max( dot( d, d ), 90000.0 ), 1e-30 );

	}

	float lightSelectPdf( sampler2D lights, uint lightCount, vec3 p, uint index ) {

		float total = 0.0;
		float mine = 0.0;
		for ( uint i = 0u; i < lightCount; i ++ ) {

			float w = lightWeight( readLightInfo( lights, i ), p );
			total += w;
			if ( i == index ) mine = w;

		}
		return total > 0.0 ? mine / total : 0.0;

	}

	float gLightSel;
	uint pickLight( sampler2D lights, uint lightCount, vec3 p, float r ) {

		float total = 0.0;
		for ( uint i = 0u; i < lightCount; i ++ ) total += lightWeight( readLightInfo( lights, i ), p );
		float t = r * total;
		float acc = 0.0;
		for ( uint i = 0u; i < lightCount; i ++ ) {

			float w = lightWeight( readLightInfo( lights, i ), p );
			acc += w;
			if ( t < acc || i == lightCount - 1u ) {

				gLightSel = total > 0.0 ? w / total : 0.0;
				return i;

			}

		}
		gLightSel = 0.0;
		return 0u;

	}

	LightRecord randomLightSample( sampler2D lights, sampler2DArray iesProfiles, uint lightCount, vec3 rayOrigin, vec3 ruv ) {`,
  ],
  [
    "uint l = uint( ruv.x * float( lightCount ) );",
    1,
    "uint l = pickLight( lights, lightCount, rayOrigin, ruv.x );",
  ],
  [
    "float lightPdf = lightRec.pdf / lightsDenom;",
    1,
    "float lightPdf = lightRec.pdf * lightSel * gLightSel;",
  ],
  [
    "misHeuristic( scatterRec.pdf, lightRec.pdf / lightsDenom )",
    1,
    "misHeuristic( scatterRec.pdf, lightRec.pdf * lightSel * lightSelectPdf( lights.tex, lights.count, ray.origin, i ) )",
  ],
  ["envPdf /= lightsDenom;", 2, "envPdf *= envSel;"],
  // 2. Clamped indirect light: next-event estimation past the first hit, and the sky, the sun
  // and emitters reached by a bounce.
  [
    "gl_FragColor.rgb += directLightContribution( - ray.direction, surf, state, hitPoint );",
    1,
    `vec3 nee = directLightContribution( - ray.direction, surf, state, hitPoint );
						gl_FragColor.rgb += state.depth > 1u ? clampMax( nee, clampIndirect ) : nee;`,
  ],
  [
    "gl_FragColor.rgb += environmentIntensity * envColor * state.throughputColor * misWeight;",
    1,
    "gl_FragColor.rgb += clampMax( environmentIntensity * envColor * state.throughputColor * misWeight, clampIndirect );",
  ],
  [
    "gl_FragColor.rgb += ( surf.emission * state.throughputColor );",
    1,
    "gl_FragColor.rgb += state.depth > 1u ? clampMax( surf.emission * state.throughputColor, clampIndirect ) : surf.emission * state.throughputColor;",
  ],
  // 3. Diffuse transmission: the flag, read from castShadow = 2.
  ["bool castShadow;", 1, "bool castShadow;\n\t\tbool diffuseTrans;"],
  [
    "m.castShadow = bool( s14.g );",
    1,
    "m.castShadow = bool( s14.g );\n\t\tm.diffuseTrans = s14.g > 1.5;",
  ],
  [
    "surf.thinFilm = material.thinFilm;",
    1,
    "surf.thinFilm = material.thinFilm;\n\t\tsurf.diffuseTrans = material.diffuseTrans;",
  ],
  // Its reflection isn't dimmed by what it transmits (the two are separate, as a leaf's are).
  [
    "float transFactor = ( 1.0 - surf.transmission );",
    1,
    "float transFactor = surf.diffuseTrans ? 1.0 : ( 1.0 - surf.transmission );",
  ],
  [
    "color *= 1.0 - surf.transmission;",
    1,
    "color *= surf.diffuseTrans ? 1.0 : 1.0 - surf.transmission;",
  ],
  // The lobes: mostly diffuse, a quarter specular, the rest out the back.
  [
    "transmissionWeight = transmission * ( 1.0 - transSpecularProb );",
    1,
    `transmissionWeight = transmission * ( 1.0 - transSpecularProb );
		if ( surf.diffuseTrans ) {

			diffuseWeight = 0.75 / ( 1.0 + transmission );
			specularWeight = 0.25;
			transmissionWeight = 0.75 * transmission / ( 1.0 + transmission );

		}`,
  ],
  // Its value and pdf (cosine-weighted, tinted by the sheen colour)…
  [
    // (Twice: the library keeps a commented-out GGX version; editing it is harmless.)
    "color = surf.transmission * surf.color;",
    2,
    `if ( surf.diffuseTrans ) {

			color = surf.transmission * surf.color * surf.sheenColor * abs( wi.z ) / PI;
			return abs( wi.z ) / PI;

		}
		color = surf.transmission * surf.color;`,
  ],
  // …and its direction.
  [
    "vec3 transmissionDirection( vec3 wo, SurfaceRecord surf ) {",
    2,
    `vec3 transmissionDirection( vec3 wo, SurfaceRecord surf ) {

		if ( surf.diffuseTrans ) {

			vec3 d = sampleSphere( rand2( 13 ) );
			d.z += 1.0;
			d = normalize( d );
			d.z = - d.z;
			return d;

		}
`,
  ],
  // The sun and sky reach it through its back, traced from that side.
  [
    "bool isSampleBelowSurface = ! surf.volumeParticle && dot( surf.faceNormal, lightRec.direction ) < 0.0;",
    1,
    "bool isSampleBelowSurface = ! surf.volumeParticle && ! surf.diffuseTrans && dot( surf.faceNormal, lightRec.direction ) < 0.0;",
  ],
  [
    "bool isSampleBelowSurface = ! surf.volumeParticle && dot( surf.faceNormal, envDirection ) < 0.0;",
    1,
    "bool isSampleBelowSurface = ! surf.volumeParticle && ! surf.diffuseTrans && dot( surf.faceNormal, envDirection ) < 0.0;",
  ],
  [
    "lightRay.origin = rayOrigin;",
    1,
    "lightRay.origin = surf.diffuseTrans ? ( dot( surf.faceNormal, lightRec.direction ) < 0.0 ? dtBack : dtFront ) : rayOrigin;",
  ],
  [
    "envRay.origin = rayOrigin;",
    1,
    "envRay.origin = surf.diffuseTrans ? ( dot( surf.faceNormal, envDirection ) < 0.0 ? dtBack : dtFront ) : rayOrigin;",
  ],
  [
    "vec3 hitPoint = stepRayOrigin( ray.origin, ray.direction, isBelowSurface ? - surf.faceNormal : surf.faceNormal, surfaceHit.dist );",
    1,
    `vec3 hitPoint = stepRayOrigin( ray.origin, ray.direction, isBelowSurface ? - surf.faceNormal : surf.faceNormal, surfaceHit.dist );
						dtFront = stepRayOrigin( ray.origin, ray.direction, surf.faceNormal, surfaceHit.dist );
						dtBack = stepRayOrigin( ray.origin, ray.direction, - surf.faceNormal, surfaceHit.dist );`,
  ],
  // Shadow rays don't pass straight through it (its light comes out diffuse).
  [
    "float transmissionFactor = ( 1.0 - metalness ) * transmission;",
    1,
    "float transmissionFactor = material.diffuseTrans ? 0.0 : ( 1.0 - metalness ) * transmission;",
  ],
];

/** The SurfaceRecord struct gets the flag too (its `bool thinFilm;` is the second one in
 * the shader, after the Material struct's). */
function addSurfaceFlag(s: string): string | null {
  const anchor = "bool thinFilm;";
  const first = s.indexOf(anchor);
  const second = first < 0 ? -1 : s.indexOf(anchor, first + anchor.length);
  if (second < 0 || s.indexOf(anchor, second + anchor.length) >= 0) return null;
  const at = second + anchor.length;
  return s.slice(0, at) + "\n\t\tbool diffuseTrans;" + s.slice(at);
}

/** The shader with every edit made, or the reasons it couldn't be. */
export function patchShader(src: string): { shader: string | null; missing: string[] } {
  let s = src;
  const missing: string[] = [];
  for (const [find, count, replace] of EDITS) {
    const n = s.split(find).length - 1;
    if (n !== count) {
      missing.push(`${n}× (wanted ${count}) ${find.slice(0, 60)}`);
      continue;
    }
    s = s.split(find).join(replace);
  }
  const flagged = addSurfaceFlag(s);
  if (!flagged) missing.push("SurfaceRecord bool thinFilm;");
  return missing.length ? { shader: null, missing } : { shader: flagged, missing };
}

export interface TransportSettings {
  /** The sky's share of light samples when there are fixtures (0–1). */
  envPick: number;
  /** The brightest indirect sample, in scene radiance. */
  clampIndirect: number;
}

/** Patches the path tracer's material in place; false (and the stock tracer) if the library
 * isn't the version these edits were written for. */
export function patchTracer(material: THREE.ShaderMaterial, o: TransportSettings): boolean {
  const { shader, missing } = patchShader(material.fragmentShader);
  if (!shader) {
    console.warn(`Render: path tracer patches not applied (${missing.join("; ")})`);
    return false;
  }
  material.fragmentShader = shader;
  material.uniforms.envPick = { value: o.envPick };
  material.uniforms.clampIndirect = { value: o.clampIndirect };
  material.needsUpdate = true;
  return true;
}

/** Sets the patched uniforms again (a later render's settings). */
export function setTransport(material: THREE.ShaderMaterial, o: TransportSettings) {
  if (material.uniforms.envPick) material.uniforms.envPick.value = o.envPick;
  if (material.uniforms.clampIndirect) material.uniforms.clampIndirect.value = o.clampIndirect;
}

/** A leaf's or a grass blade's material for the path tracer (ADR-119): it reflects its own
 * albedo and scatters `transmission` of that again out of its back, tinted (a green leaf
 * transmits yellower than it reflects: Jacquemoud & Ustin's measurements, PROSPECT), and
 * shades as opaque for shadows. `castShadow = 2` is the patched shader's flag. */
/** Off: leaves and grass opaque (a development aid for comparing). */
export const foliage = { translucent: true };

export function translucent(
  m: THREE.MeshPhysicalMaterial,
  transmission: number,
  tint: [number, number, number] = LEAF_TINT,
): THREE.MeshPhysicalMaterial {
  if (!foliage.translucent) return m;
  m.transmission = transmission;
  m.thickness = 0;
  m.ior = 1.45;
  m.sheen = 0;
  m.sheenColor.setRGB(tint[0], tint[1], tint[2]);
  (m as unknown as { castShadow: number }).castShadow = 2;
  return m;
}

/** What leaves transmit relative to what they reflect, per channel. */
export const LEAF_TINT: [number, number, number] = [1.1, 1.0, 0.55];
