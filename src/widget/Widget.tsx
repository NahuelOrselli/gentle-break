import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import abuelaUrl from "../assets/abuela.png";
import mustacheUrl from "../assets/mustache.webp";
import { Hourglass, type GlassMood } from "../hourglass/Hourglass.tsx";
import { PixelClock } from "../PixelClock.tsx";
import { getState, isTauri, pad, startDragging, togglePanel } from "../platform.ts";
import {
	clockMetrics,
	hasVisualReaction,
	isFaceReaction,
	type FaceReaction,
	type Reaction,
	type WidgetSize,
} from "../types.ts";

const FACE_IMAGE: Record<FaceReaction, string> = {
	mustache: mustacheUrl,
	abuela: abuelaUrl,
};

interface LiveState {
	dnd: boolean;
	widgetVisible: boolean;
	widgetSize: WidgetSize;
	reaction: Reaction;
}

interface Shout {
	title: string;
	body: string;
	side: "left" | "right";
	vertical: "above" | "below";
	face: FaceReaction;
}

const DRAG_THRESHOLD_PX = 3;
const SHOUT_FADE_MS = 280;

function afterFrames(count: number): Promise<void> {
	return new Promise((resolve) => {
		let left = count;
		const step = () => {
			left -= 1;
			if (left <= 0) resolve();
			else requestAnimationFrame(step);
		};
		requestAnimationFrame(step);
	});
}

function BubbleTail({ side }: { side: Shout["side"] }) {
	const stroke = "rgba(255, 45, 122, 0.55)";
	const fill = "#140c10";
	// Globo a la izquierda del personaje → punta hacia la derecha (sin borde en el lado que toca el globo).
	if (side === "left") {
		return (
			<svg className="bubble-tail" viewBox="0 0 9 14" width="9" height="14" aria-hidden="true">
				<polygon points="1,1 1,13 9,7" fill={fill} />
				<path d="M1 1.5 L1 12.5" fill="none" stroke={fill} strokeWidth="2" />
				<path d="M1 1.5 L9 7 L1 12.5" fill="none" stroke={stroke} strokeWidth="1" strokeLinejoin="round" strokeLinecap="round" />
			</svg>
		);
	}
	// Globo a la derecha → punta hacia la izquierda.
	return (
		<svg className="bubble-tail" viewBox="0 0 9 14" width="9" height="14" aria-hidden="true">
			<polygon points="8,1 8,13 0,7" fill={fill} />
			<path d="M8 1.5 L8 12.5" fill="none" stroke={fill} strokeWidth="2" />
			<path d="M8 1.5 L0 7 L8 12.5" fill="none" stroke={stroke} strokeWidth="1" strokeLinejoin="round" strokeLinecap="round" />
		</svg>
	);
}

function useClock() {
	const [now, setNow] = useState(() => new Date());
	useEffect(() => {
		const id = window.setInterval(() => setNow(new Date()), 250);
		return () => window.clearInterval(id);
	}, []);
	return now;
}

/** The always-on rectangle: hourglass, pixel time, drag to move, click to open. */
export function Widget() {
	const now = useClock();
	const press = useRef<{ x: number; y: number; dragging: boolean } | undefined>(undefined);
	const pulse = useRef<number | undefined>(undefined);
	const [dnd, setDnd] = useState(false);
	const [size, setSize] = useState<WidgetSize>("large");
	const [widgetVisible, setWidgetVisible] = useState(true);
	const widgetVisibleRef = useRef(widgetVisible);
	widgetVisibleRef.current = widgetVisible;
	const [reaction, setReaction] = useState<Reaction>("mustache");
	const reactionRef = useRef(reaction);
	reactionRef.current = reaction;
	const [alert, setAlert] = useState(false);
	const [playing, setPlaying] = useState<Reaction | null>(null);
	const [playKey, setPlayKey] = useState(0);
	const [shout, setShout] = useState<Shout | null>(null);
	const [shoutStage, setShoutStage] = useState<Shout | null>(null);
	const shoutStageRef = useRef<Shout | null>(null);
	const [shoutExiting, setShoutExiting] = useState(false);
	const [shoutRevealed, setShoutRevealed] = useState(false);
	const [shoutArming, setShoutArming] = useState(false);
	const shoutRef = useRef<Shout | null>(null);
	const shoutFade = useRef<number | undefined>(undefined);
	const playGen = useRef(0);
	shoutStageRef.current = shoutStage;

	useEffect(() => {
		let cancel = false;
		const stops: Array<() => void> = [];
		void getState()
			.then((settings) => {
				if (cancel) return;
				setDnd(settings.dnd);
				setWidgetVisible(settings.widgetVisible);
				setSize(settings.widgetSize);
				setReaction(settings.reaction);
			})
			.catch(() => undefined);

		const applyLive = (live: LiveState) => {
			setDnd(live.dnd);
			if (typeof live.widgetVisible === "boolean") setWidgetVisible(live.widgetVisible);
			setSize(live.widgetSize);
			if (live.reaction) setReaction(live.reaction);
		};

		if (!isTauri()) {
			const onLive = (event: Event) => {
				const detail = (event as CustomEvent<LiveState>).detail;
				if (detail && typeof detail.dnd === "boolean") applyLive(detail);
			};
			window.addEventListener("gentle-break-live", onLive);
			return () => {
				cancel = true;
				window.removeEventListener("gentle-break-live", onLive);
			};
		}

		void (async () => {
			const { listen } = await import("@tauri-apps/api/event");
			const { invoke } = await import("@tauri-apps/api/core");

			const clearShoutNow = async () => {
				if (shoutFade.current) {
					window.clearTimeout(shoutFade.current);
					shoutFade.current = undefined;
				}
				setShoutExiting(false);
				setShoutRevealed(false);
				setShoutArming(false);
				setShout(null);
				shoutRef.current = null;
				await invoke("end_shout").catch(() => undefined);
				setShoutStage(null);
			};

			const fadeOutShout = () => {
				if (!shoutRef.current) return;
				if (shoutFade.current) window.clearTimeout(shoutFade.current);
				setShoutExiting(true);
				shoutFade.current = window.setTimeout(() => {
					shoutFade.current = undefined;
					setShoutExiting(false);
					setShout(null);
					shoutRef.current = null;
					void invoke("end_shout")
						.catch(() => undefined)
						.finally(() => setShoutStage(null));
				}, SHOUT_FADE_MS);
			};

			const play = async (reaction: Reaction, title: string, body: string) => {
				if (!widgetVisibleRef.current || !hasVisualReaction(reaction)) return;
				const generation = ++playGen.current;
				try {
					if (pulse.current) window.clearTimeout(pulse.current);
					const wasShout = shoutRef.current !== null || shoutFade.current !== undefined;
					if (wasShout && !isFaceReaction(reaction)) {
						await clearShoutNow();
					}
					if (cancel || playGen.current !== generation) return;
					if (isFaceReaction(reaction)) {
						if (wasShout) await clearShoutNow();
						if (cancel || playGen.current !== generation) return;
						const layout = await invoke<{ side: string; vertical: string }>("peek_shout_layout");
						const next: Shout = {
							title,
							body,
							side: layout.side === "left" ? "left" : "right",
							vertical: layout.vertical === "below" ? "below" : "above",
							face: reaction,
						};
						shoutRef.current = next;
						flushSync(() => {
							setShoutArming(true);
							setShoutRevealed(false);
							setShoutStage(next);
							setShout(next);
						});
						await invoke("begin_shout");
						if (cancel || playGen.current !== generation) {
							await invoke("end_shout");
							flushSync(() => {
								setShoutArming(false);
								setShoutStage(null);
								setShout(null);
								shoutRef.current = null;
							});
							return;
						}
						await afterFrames(2);
						if (cancel || playGen.current !== generation) {
							await invoke("end_shout");
							flushSync(() => {
								setShoutArming(false);
								setShoutStage(null);
								setShout(null);
								shoutRef.current = null;
							});
							return;
						}
						flushSync(() => setShoutArming(false));
						requestAnimationFrame(() => {
							if (playGen.current !== generation) return;
							setShoutRevealed(true);
						});
					} else {
						shoutRef.current = null;
						setShout(null);
						setShoutStage(null);
					}
					if (!isFaceReaction(reaction)) setPlayKey(generation);
					setPlaying(reaction);
					if (!isFaceReaction(reaction)) setAlert(true);
					pulse.current = window.setTimeout(() => {
						if (playGen.current !== generation) return;
						if (!isFaceReaction(reaction)) setAlert(false);
						setPlaying(null);
						if (isFaceReaction(reaction)) fadeOutShout();
					}, 7000);
				} catch {
					if (playGen.current !== generation) return;
					setAlert(false);
					setPlaying(null);
					await clearShoutNow();
				}
			};

			const stopDnd = await listen<LiveState>("gentle-break://live", (event) => applyLive(event.payload));
			const stopFired = await listen<{ title: string; body: string }>("gentle-break://fired", (event) => {
				void play(reactionRef.current, event.payload.title, event.payload.body);
			});
			const stopPreview = await listen<{ reaction: Reaction; title: string; body: string }>("gentle-break://preview", (event) => {
				void play(event.payload.reaction, event.payload.title, event.payload.body);
			});
			if (cancel) {
				stopDnd();
				stopFired();
				stopPreview();
				return;
			}
			stops.push(stopDnd, stopFired, stopPreview);
		})();

		return () => {
			cancel = true;
			playGen.current += 1;
			for (const stop of stops) stop();
			if (pulse.current) window.clearTimeout(pulse.current);
			if (shoutFade.current) window.clearTimeout(shoutFade.current);
			if (shoutRef.current || shoutStageRef.current) {
				shoutRef.current = null;
				shoutStageRef.current = null;
				void import("@tauri-apps/api/core").then(({ invoke }) => invoke("end_shout"));
			}
		};
	}, []);

	const hh = pad(now.getHours());
	const mm = pad(now.getMinutes());
	const colon = now.getSeconds() % 2 === 0 ? ":" : " ";
	let mood: GlassMood = "idle";
	if (dnd) mood = "dnd";
	else if (alert) mood = "alert";
	const label = `Gentle Break, ${hh}:${mm}${dnd ? ", no molestar" : ""}. Arrastrá para mover, clic para abrir.`;
	const clock = clockMetrics(size);
	const motion = playing && !isFaceReaction(playing) ? playing : "";
	const layout = shoutStage;

	return (
		<div
			className={`stage${layout ? ` stage-shout stage-${layout.side} stage-${layout.vertical}${shoutArming ? " is-arming" : ""}` : ""}`}
			data-size={size}
		>
			{shout ? (
				<div
					className={`shout-cluster${shoutRevealed ? " is-revealed" : " is-entering"}${shoutExiting ? " is-exiting" : ""}`}
					data-face={shout.face}
				>
					<div className="mustache-stack">
						<img className="mustache" src={FACE_IMAGE[shout.face]} alt="" decoding="async" />
					</div>
					<div className="bubble" role="status">
						<strong>{shout.title}</strong>
						{shout.body ? <span>{shout.body}</span> : null}
						<BubbleTail side={shout.side} />
					</div>
				</div>
			) : null}
		<button
			type="button"
			className={`float float-${mood}${motion ? ` float-${motion}` : ""}`}
			data-size={size}
			aria-label={label}
			onContextMenu={(event) => event.preventDefault()}
			onPointerDown={(event) => {
				if (event.button !== 0) return;
				press.current = { x: event.clientX, y: event.clientY, dragging: false };
			}}
			onPointerMove={(event) => {
				const current = press.current;
				if (!current || current.dragging) return;
				if (Math.hypot(event.clientX - current.x, event.clientY - current.y) > DRAG_THRESHOLD_PX) {
					current.dragging = true;
					void startDragging();
				}
			}}
			onPointerUp={() => {
				const current = press.current;
				press.current = undefined;
				if (current && !current.dragging) void togglePanel();
			}}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					void togglePanel();
				}
			}}
		>
			<span className="float-card" key={motion ? playKey : undefined}>
				<span className="glass-box">
					<Hourglass mood={mood} />
				</span>
				<PixelClock text={`${hh}${colon}${mm}`} pixel={clock.pixel} gap={clock.gap} letterGap={clock.letterGap} />
			</span>
		</button>
		</div>
	);
}
