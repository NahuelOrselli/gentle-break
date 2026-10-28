/** A centerline of the hourglass. `length` is in viewBox pixels and only sets how long the light takes. */
export interface GlassStroke {
	length: number;
	d: string;
}

/** The crop that holds the hourglass and its sparkles. */
export const GLASS_VIEWBOX = "0 0 240 360";

/** Longest strokes first, same ordering idea as the rose. */
export const GLASS_STROKES: readonly GlassStroke[] = [
	{ length: 184, d: "M28 22 H212" },
	{ length: 184, d: "M28 334 H212" },
	{ length: 160, d: "M48 46 C44 108 84 146 108 168" },
	{ length: 160, d: "M192 46 C196 108 156 146 132 168" },
	{ length: 160, d: "M108 188 C84 214 44 256 48 310" },
	{ length: 160, d: "M132 188 C156 214 196 256 192 310" },
	{ length: 152, d: "M44 40 H196" },
	{ length: 152, d: "M44 316 H196" },
	{ length: 140, d: "M58 308 Q120 268 182 308" },
	{ length: 130, d: "M64 58 C66 104 96 140 112 162" },
	{ length: 130, d: "M176 58 C174 104 144 140 128 162" },
	{ length: 130, d: "M112 194 C96 222 66 258 64 300" },
	{ length: 130, d: "M128 194 C144 222 174 258 176 300" },
	{ length: 120, d: "M72 292 Q120 258 168 292" },
	{ length: 116, d: "M70 90 Q120 118 170 90" },
	{ length: 96, d: "M82 112 Q120 134 158 112" },
	{ length: 88, d: "M120 168 V256" },
	{ length: 54, d: "M114 178 V232" },
	{ length: 54, d: "M126 178 V232" },
	{ length: 36, d: "M108 166 V190" },
	{ length: 36, d: "M132 166 V190" },
	{ length: 28, d: "M28 22 V50" },
	{ length: 28, d: "M212 22 V50" },
	{ length: 28, d: "M28 306 V334" },
	{ length: 28, d: "M212 306 V334" },
];

/** How many of the longest strokes carry the traveling light. Shorter ones would flicker as dots. */
export const GLASS_LIGHT_STROKES = 17;

/** Sparkles around the glass as [cx, cy, r]. */
export const GLASS_SPARKLES: readonly (readonly [number, number, number])[] = [
	[22, 78, 2.4],
	[218, 96, 1.8],
	[16, 188, 2.6],
	[224, 210, 2],
	[46, 14, 1.6],
	[196, 348, 2.2],
	[34, 276, 1.8],
	[206, 64, 2.4],
	[120, 8, 1.5],
	[86, 348, 2],
	[150, 12, 1.7],
	[230, 150, 1.6],
];
