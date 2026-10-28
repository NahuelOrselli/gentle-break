import type { CSSProperties } from "react";
import { GLASS_LIGHT_STROKES, GLASS_SPARKLES, GLASS_STROKES, GLASS_VIEWBOX } from "./strokes.ts";

/** Idle breathes and runs the light. Alert speeds it up. Do-not-disturb dims it. */
export type GlassMood = "idle" | "alert" | "dnd";

const outline = GLASS_STROKES.map(({ d }) => <path key={d} d={d} />);

const lights = GLASS_STROKES.slice(0, GLASS_LIGHT_STROKES).map(({ d, length }, i) => {
	// Longer strokes take longer to travel, so the light moves at a similar speed everywhere.
	const duration = Math.min(4.2, Math.max(1.1, length / 120)).toFixed(2);
	const style = { "--i": i, "--d": `${duration}s` } as CSSProperties;
	return <path key={d} d={d} pathLength={1} style={style} />;
});

const sparkles = GLASS_SPARKLES.map(([cx, cy, r], i) => (
	<circle key={`${cx},${cy}`} cx={cx} cy={cy} r={r} style={{ "--i": i } as CSSProperties} />
));

/**
 * The neon hourglass: a glowing outline, a light layer, and sparkles, stacked as
 * three inline SVGs. Each layer is its own SVG root because WebKit (the macOS
 * webview) ignores CSS filters on elements inside an SVG, so the glow must
 * sit on an outer `<svg>` box. Same structure as Gentle Dot's rose.
 */
export function Hourglass({ mood }: { mood: GlassMood }) {
	return (
		<span className={`glass glass-${mood}`} aria-hidden="true">
			<svg className="glass-base" viewBox={GLASS_VIEWBOX} aria-hidden="true" focusable="false">
				{outline}
			</svg>
			<svg className="glass-light" viewBox={GLASS_VIEWBOX} aria-hidden="true" focusable="false">
				{lights}
			</svg>
			<svg className="glass-sparkles" viewBox={GLASS_VIEWBOX} aria-hidden="true" focusable="false">
				{sparkles}
			</svg>
		</span>
	);
}
