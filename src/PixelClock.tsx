import { GLYPHS } from "./pixelFont.ts";

interface PixelClockProps {
	text: string;
	pixel?: number;
	gap?: number;
	letterGap?: number;
}

/** Current time drawn as a pixel grid, not a circle and not a UI font. */
export function PixelClock({ text, pixel = 3, gap = 1, letterGap = 3 }: PixelClockProps) {
	const pitch = pixel + gap;
	const rects: { x: number; y: number; key: string }[] = [];
	let cursor = 0;

	for (const char of text) {
		const rows = GLYPHS[char] ?? GLYPHS[" "];
		rows.forEach((row, y) => {
			for (let col = 0; col < row.length; col += 1) {
				if (row[col] === "1") {
					rects.push({ x: cursor + col * pitch, y: y * pitch, key: `${cursor}-${col}-${y}` });
				}
			}
		});
		const cols = rows[0]?.length ?? 0;
		cursor += cols * pitch - gap + letterGap;
	}

	const width = Math.max(pixel, cursor - letterGap);
	const height = 7 * pitch - gap;

	return (
		<svg
			className="pixel-clock"
			width={width}
			height={height}
			viewBox={`0 0 ${width} ${height}`}
			aria-hidden="true"
			focusable="false"
		>
			{rects.map((rect) => (
				<rect key={rect.key} x={rect.x} y={rect.y} width={pixel} height={pixel} />
			))}
		</svg>
	);
}
