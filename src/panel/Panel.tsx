import { useEffect, useRef, useState } from "react";
import { PixelClock } from "../PixelClock.tsx";
import { chooseSoundFile, getState, hidePanel, importSound, isTauri, pad, previewReaction, previewSound, quitApp, removeSound, saveState } from "../platform.ts";
import {
	PRESET_SOUNDS,
	REACTIONS,
	WEEKDAYS,
	WIDGET_SIZES,
	type Alarm,
	type Break,
	type BreakWindow,
	type CustomSound,
	hasVisualReaction,
	isFaceReaction,
	type Reaction,
	type Settings,
	type WidgetSize,
} from "../types.ts";

type PanelTab = "alarms" | "breaks" | "config";

const TABS: { id: PanelTab; label: string }[] = [
	{ id: "alarms", label: "Alarmas" },
	{ id: "breaks", label: "Breaks" },
	{ id: "config", label: "Config" },
];

function useClock() {
	const [now, setNow] = useState(() => new Date());
	useEffect(() => {
		const id = window.setInterval(() => setNow(new Date()), 250);
		return () => window.clearInterval(id);
	}, []);
	return now;
}

function minutesOf(value: string): number | null {
	const match = /^(\d{1,2}):(\d{2})$/.exec(value);
	if (!match) return null;
	const hour = Number(match[1]);
	const minute = Number(match[2]);
	if (hour > 23 || minute > 59) return null;
	return hour * 60 + minute;
}

function formatMinutes(total: number): string {
	return `${pad(Math.floor(total / 60))}:${pad(total % 60)}`;
}

/** Times a break can ring inside the global window. The start itself is not one of them. */
export function breakMoments(window: BreakWindow, everyMinutes: number): string[] {
	const start = minutesOf(window.start);
	const end = minutesOf(window.end);
	if (start == null || end == null || end <= start || everyMinutes < 1) return [];
	const moments: string[] = [];
	for (let cursor = start + everyMinutes; cursor < end && moments.length < 8; cursor += everyMinutes) {
		moments.push(formatMinutes(cursor));
	}
	const more = start + everyMinutes * (moments.length + 1) < end;
	if (more) moments.push("…");
	return moments;
}

function toggleDay(days: number[], day: number): number[] {
	const next = days.includes(day) ? days.filter((item) => item !== day) : [...days, day];
	return next.sort((a, b) => a - b);
}

function PlayIcon() {
	return (
		<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
			<path d="M5 3.2v9.6L13 8 5 3.2z" fill="currentColor" />
		</svg>
	);
}

function UploadIcon() {
	return (
		<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
			<path d="M8 2.2 4.2 6h2.3v4.2h3V6h2.3L8 2.2zM3 12.2h10V14H3v-1.8z" fill="currentColor" />
		</svg>
	);
}

function PlusIcon() {
	return (
		<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
			<path d="M7.2 2.4h1.6v4.4H13v1.6H8.8V13H7.2V8.4H3V6.8h4.2V2.4z" fill="currentColor" />
		</svg>
	);
}

function SoundSelect({
	value,
	sounds,
	onChange,
}: {
	value: string;
	sounds: CustomSound[];
	onChange: (sound: string) => void;
}) {
	const options = [
		...PRESET_SOUNDS,
		...sounds.map((sound) => ({ id: `custom:${sound.id}`, label: sound.name })),
	];
	const [open, setOpen] = useState(false);
	const root = useRef<HTMLDivElement>(null);
	const [menu, setMenu] = useState<{ top: number; left: number; width: number } | null>(null);
	const current = options.find((option) => option.id === value);

	useEffect(() => {
		if (!open) return;
		const close = (event: PointerEvent) => {
			if (!root.current?.contains(event.target as Node)) setOpen(false);
		};
		window.addEventListener("pointerdown", close);
		return () => window.removeEventListener("pointerdown", close);
	}, [open]);

	return (
		<div className="field" ref={root}>
			<span className="field-label">Sonido</span>
			<button
				type="button"
				className="field-select"
				aria-label="Sonido"
				aria-expanded={open}
				onClick={(event) => {
					const rect = event.currentTarget.getBoundingClientRect();
					setMenu({ top: rect.bottom + 6, left: rect.left, width: rect.width });
					setOpen((valueOpen) => !valueOpen);
				}}
			>
				{current?.label ?? "Cristal"}
			</button>
			{open && menu ? (
				<ul className="field-menu" role="listbox" style={{ top: menu.top, left: menu.left, width: menu.width }}>
					{options.map((option) => (
						<li key={option.id}>
							<button
								type="button"
								role="option"
								aria-selected={option.id === value}
								className={option.id === value ? "on" : ""}
								onClick={() => {
									onChange(option.id);
									setOpen(false);
								}}
							>
								{option.label}
							</button>
						</li>
					))}
				</ul>
			) : null}
		</div>
	);
}

function DayPicker({ days, onChange }: { days: number[]; onChange: (days: number[]) => void }) {
	return (
		<div className="days" role="group" aria-label="Días">
			{WEEKDAYS.map((day) => {
				const on = days.includes(day.id);
				return (
					<button
						key={day.id}
						type="button"
						className={on ? "day on" : "day"}
						aria-pressed={on}
						aria-label={day.name}
						onClick={() => onChange(toggleDay(days, day.id))}
					>
						{day.label}
					</button>
				);
			})}
		</div>
	);
}

export function Panel() {
	const now = useClock();
	const [settings, setSettings] = useState<Settings | null>(null);
	const [tab, setTab] = useState<PanelTab>("alarms");
	const tabScroll = useRef<HTMLDivElement>(null);
	const [error, setError] = useState<string | null>(null);
	const lastPreview = useRef(0);
	const faceAsBreak = useRef(false);

	const showReaction = (reaction: Reaction) => {
		if (!hasVisualReaction(reaction)) return;
		const now = Date.now();
		if (now - lastPreview.current < 400) return;
		lastPreview.current = now;
		const asBreak = isFaceReaction(reaction) && faceAsBreak.current;
		if (isFaceReaction(reaction)) faceAsBreak.current = !faceAsBreak.current;
		void previewReaction(reaction, asBreak ? "Caminar" : "Almuerzo", asBreak ? "Parate a caminar" : "").catch((reason: unknown) =>
			setError(reason instanceof Error ? reason.message : "No pude probar la animación"),
		);
	};

	useEffect(() => {
		let cancel = false;
		const stops: Array<() => void> = [];
		void getState()
			.then((loaded) => {
				if (!cancel) setSettings(loaded);
			})
			.catch((reason: unknown) => {
				if (!cancel) setError(reason instanceof Error ? reason.message : "No pude leer la configuración");
			});

		if (isTauri()) {
			void (async () => {
				const { listen } = await import("@tauri-apps/api/event");
				const stop = await listen<{ dnd: boolean; widgetVisible: boolean; widgetSize: WidgetSize; reaction: Reaction }>("gentle-break://live", (event) => {
					setSettings((current) => {
						if (!current) return current;
						const next = event.payload;
						if (
							current.dnd === next.dnd &&
							current.widgetVisible === next.widgetVisible &&
							current.widgetSize === next.widgetSize &&
							current.reaction === next.reaction
						) {
							return current;
						}
						return {
							...current,
							dnd: next.dnd,
							widgetVisible: next.widgetVisible,
							widgetSize: next.widgetSize,
							reaction: next.reaction,
						};
					});
				});
				if (cancel) stop();
				else stops.push(stop);
			})();
		}

		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") void hidePanel();
		};
		window.addEventListener("keydown", onKey);
		return () => {
			cancel = true;
			for (const stop of stops) stop();
			window.removeEventListener("keydown", onKey);
		};
	}, []);

	useEffect(() => {
		tabScroll.current?.scrollTo({ top: 0 });
	}, [tab]);

	useEffect(() => {
		if (!settings) return;
		const id = window.setTimeout(() => {
			void saveState(settings).catch((reason: unknown) => {
				setError(reason instanceof Error ? reason.message : "No pude guardar");
			});
		}, 200);
		return () => window.clearTimeout(id);
	}, [settings]);

	if (!settings) {
		return (
			<div className="panel-shell">
				<section className="panel">
					<p className="muted">Cargando…</p>
				</section>
			</div>
		);
	}

	const start = minutesOf(settings.breakWindow.start);
	const end = minutesOf(settings.breakWindow.end);
	const windowInvalid = start == null || end == null || end <= start;
	const hh = pad(now.getHours());
	const mm = pad(now.getMinutes());
	const ss = pad(now.getSeconds());
	const date = new Intl.DateTimeFormat("es-AR", {
		weekday: "long",
		day: "numeric",
		month: "long",
	}).format(now);

	function patch(next: Settings) {
		setSettings(next);
		setError(null);
	}

	function updateAlarm(id: string, alarm: Alarm) {
		if (!settings) return;
		patch({ ...settings, alarms: settings.alarms.map((item) => (item.id === id ? alarm : item)) });
	}

	function updateBreak(id: string, item: Break) {
		if (!settings) return;
		patch({ ...settings, breaks: settings.breaks.map((current) => (current.id === id ? item : current)) });
	}

	return (
		<div className="panel-shell">
			<section className="panel">
				<header className="panel-head">
					<div>
						<p className="eyebrow">Gentle Break</p>
						<h1>Parate un rato</h1>
					</div>
					<button type="button" className="icon-button" aria-label="Cerrar" onClick={() => void hidePanel()}>
						×
					</button>
				</header>

				<div className="hero">
					<PixelClock text={`${hh}:${mm}`} pixel={8} gap={2} letterGap={6} />
					<PixelClock text={ss} pixel={3} gap={1} letterGap={2} />
					<p className="date">{date}</p>
				</div>

				<div className="tabs" role="tablist" aria-label="Secciones">
					{TABS.map((item) => (
						<button
							key={item.id}
							type="button"
							role="tab"
							aria-selected={tab === item.id}
							className={tab === item.id ? "tab on" : "tab"}
							onClick={() => setTab(item.id)}
						>
							{item.label}
						</button>
					))}
				</div>

				<div className="tab-scroll" ref={tabScroll}>
				{tab === "alarms" ? (
				<section className="block">
					<div className="block-head">
						<h2>Alarmas</h2>
						<button
							type="button"
							onClick={() =>
								patch({
									...settings,
									alarms: [
										...settings.alarms,
										{
											id: crypto.randomUUID(),
											name: "Nueva alarma",
											time: "12:00",
											days: [1, 2, 3, 4, 5],
											enabled: true,
											sound: "glass",
										},
									],
								})
							}
							className="add-button"
						>
							<PlusIcon />
							Nueva alarma
						</button>
					</div>
					<p className="hint">Suena a la hora elegida, los días marcados.</p>
					{settings.alarms.length === 0 ? <p className="empty">Todavía no hay alarmas.</p> : null}
					{settings.alarms.map((alarm) => (
						<article key={alarm.id} className={alarm.enabled ? "card" : "card off"}>
							<div className="card-row">
								<input
									aria-label="Nombre de la alarma"
									value={alarm.name}
									maxLength={80}
									onChange={(event) => updateAlarm(alarm.id, { ...alarm, name: event.target.value })}
								/>
								<input
									aria-label="Hora de la alarma"
									type="time"
									value={alarm.time}
									onChange={(event) => updateAlarm(alarm.id, { ...alarm, time: event.target.value })}
								/>
							</div>
							<DayPicker days={alarm.days} onChange={(days) => updateAlarm(alarm.id, { ...alarm, days })} />
							<SoundSelect
								value={alarm.sound || "glass"}
								sounds={settings.sounds ?? []}
								onChange={(sound) => updateAlarm(alarm.id, { ...alarm, sound })}
							/>
							{alarm.days.length === 0 ? <p className="warn">Elegí al menos un día.</p> : null}
							<div className="card-actions">
								<button
									type="button"
									className={alarm.enabled ? "chip on" : "chip"}
									aria-pressed={alarm.enabled}
									onClick={() => updateAlarm(alarm.id, { ...alarm, enabled: !alarm.enabled })}
								>
									{alarm.enabled ? "Activa" : "Pausada"}
								</button>
								<button
									type="button"
									className="text-button"
									onClick={() => patch({ ...settings, alarms: settings.alarms.filter((item) => item.id !== alarm.id) })}
								>
									Eliminar
								</button>
							</div>
						</article>
					))}
				</section>
				) : null}

				{tab === "breaks" ? (
				<section className="block">
					<div className="block-head">
						<h2>Breaks</h2>
						<button
							type="button"
							onClick={() =>
								patch({
									...settings,
									breaks: [
										...settings.breaks,
										{
											id: crypto.randomUUID(),
											name: "Nuevo break",
											message: "Parate a caminar",
											everyMinutes: 120,
											enabled: true,
											sound: "ping",
										},
									],
								})
							}
							className="add-button"
						>
							<PlusIcon />
							Nuevo break
						</button>
					</div>
					<p className="hint">Avisos que se repiten. Solo suenan dentro de la ventana global.</p>
					<div className="window-card">
						<p className="label">Ventana global</p>
						<div className="card-row">
							<label>
								Inicio
								<input
									type="time"
									value={settings.breakWindow.start}
									onChange={(event) =>
										patch({ ...settings, breakWindow: { ...settings.breakWindow, start: event.target.value } })
									}
								/>
							</label>
							<label>
								Fin
								<input
									type="time"
									value={settings.breakWindow.end}
									onChange={(event) =>
										patch({ ...settings, breakWindow: { ...settings.breakWindow, end: event.target.value } })
									}
								/>
							</label>
						</div>
						<DayPicker
							days={settings.breakWindow.days}
							onChange={(days) => patch({ ...settings, breakWindow: { ...settings.breakWindow, days } })}
						/>
						{windowInvalid ? <p className="warn">La hora de fin tiene que ser después del inicio.</p> : null}
						{settings.breakWindow.days.length === 0 ? <p className="warn">Elegí al menos un día.</p> : null}
					</div>
					{settings.breaks.length === 0 ? <p className="empty">Todavía no hay breaks.</p> : null}
					{settings.breaks.map((item) => {
						const moments = breakMoments(settings.breakWindow, item.everyMinutes);
						return (
							<article key={item.id} className={item.enabled ? "card" : "card off"}>
								<div className="card-row">
									<input
										aria-label="Nombre del break"
										value={item.name}
										maxLength={80}
										onChange={(event) => updateBreak(item.id, { ...item, name: event.target.value })}
									/>
									<label className="every">
										Cada
										<input
											aria-label="Minutos entre avisos"
											type="number"
											min={1}
											max={720}
											value={item.everyMinutes}
											onChange={(event) => {
												const value = Number(event.target.value);
												if (!Number.isInteger(value)) return;
												updateBreak(item.id, { ...item, everyMinutes: Math.min(720, Math.max(1, value)) });
											}}
										/>
										min
									</label>
								</div>
								<SoundSelect
									value={item.sound || "ping"}
									sounds={settings.sounds ?? []}
									onChange={(sound) => updateBreak(item.id, { ...item, sound })}
								/>
								<textarea
									aria-label="Mensaje del break"
									rows={2}
									maxLength={240}
									value={item.message}
									onChange={(event) => updateBreak(item.id, { ...item, message: event.target.value })}
								/>
								<div className="presets">
									{[30, 60, 90, 120].map((minutes) => (
										<button
											key={minutes}
											type="button"
											className={item.everyMinutes === minutes ? "chip on" : "chip"}
											onClick={() => updateBreak(item.id, { ...item, everyMinutes: minutes })}
										>
											{minutes} min
										</button>
									))}
								</div>
								{moments.length > 0 ? <p className="hint">Suena a las {moments.join(", ")}</p> : null}
								<div className="card-actions">
									<button
										type="button"
										className={item.enabled ? "chip on" : "chip"}
										aria-pressed={item.enabled}
										onClick={() => updateBreak(item.id, { ...item, enabled: !item.enabled })}
									>
										{item.enabled ? "Activo" : "Pausado"}
									</button>
									<button
										type="button"
										className="text-button"
										onClick={() => patch({ ...settings, breaks: settings.breaks.filter((current) => current.id !== item.id) })}
									>
										Eliminar
									</button>
								</div>
							</article>
						);
					})}
				</section>
				) : null}

				{tab === "config" ? (
					<section className="block">
						<h2>Configuración</h2>
						<button
							type="button"
							className={settings.dnd ? "dnd on" : "dnd"}
							role="switch"
							aria-checked={settings.dnd}
							onClick={() => patch({ ...settings, dnd: !settings.dnd })}
						>
							<span className="dnd-copy">
								<strong>No molestar</strong>
								<span>Apaga todas las notificaciones, alarmas y breaks.</span>
							</span>
							<span className="switch" aria-hidden="true" />
						</button>
						<button
							type="button"
							className="dnd widget-visibility"
							onClick={() => patch({ ...settings, widgetVisible: !settings.widgetVisible })}
						>
							<span className="dnd-copy">
								<strong>{settings.widgetVisible ? "Ocultar widget" : "Mostrar widget"}</strong>
								<span>El reloj flotante desaparece, pero la app sigue en la barra de menú.</span>
							</span>
						</button>
						<div className="window-card">
							<p className="label">Tamaño del widget</p>
							<p className="hint">Grande es el tamaño de ahora.</p>
							<div className="sizes">
								{WIDGET_SIZES.map((item) => (
									<button
										key={item.id}
										type="button"
										className={settings.widgetSize === item.id ? "size on" : "size"}
										aria-pressed={settings.widgetSize === item.id}
										onClick={() => patch({ ...settings, widgetSize: item.id })}
									>
										{item.label}
									</button>
								))}
							</div>
						</div>
						<div className="window-card">
							<p className="label">Animación cuando suena</p>
							<div className="sizes">
								{REACTIONS.map((item) => (
									<button
										key={item.id}
										type="button"
										className={settings.reaction === item.id ? "size on" : "size"}
										aria-pressed={settings.reaction === item.id}
										onClick={() => {
											if (settings.reaction === item.id) showReaction(item.id);
											else patch({ ...settings, reaction: item.id });
										}}
										onDoubleClick={() => {
											if (settings.reaction !== item.id) patch({ ...settings, reaction: item.id });
											showReaction(item.id);
										}}
									>
										{item.label}
									</button>
								))}
							</div>
						</div>
						<div className="window-card">
							<div className="block-head">
								<p className="label">Sonidos</p>
								<button
									type="button"
									className="icon-action"
									onClick={() => {
										void (async () => {
											try {
												const picked = await chooseSoundFile();
												if (!picked) return;
												const imported = await importSound(picked);
												patch({ ...settings, sounds: [...(settings.sounds ?? []), imported] });
											} catch (reason) {
												setError(reason instanceof Error ? reason.message : "No pude usar ese sonido");
											}
										})();
									}}
								>
									<UploadIcon />
									Subir
								</button>
							</div>
							<p className="hint">WAV, MP3, M4A, AIFF o CAF. Hasta 15 segundos y 2 MB.</p>
							<ul className="sound-list">
								{PRESET_SOUNDS.map((sound) => (
									<li key={sound.id}>
										<span>{sound.label}</span>
										<button type="button" className="icon-action" aria-label={`Probar ${sound.label}`} onClick={() => void previewSound(sound.id)}>
											<PlayIcon />
											Probar
										</button>
									</li>
								))}
								{(settings.sounds ?? []).map((sound) => (
									<li key={sound.id}>
										<span>{sound.name}</span>
										<span className="sound-actions">
											<button type="button" className="icon-action" aria-label={`Probar ${sound.name}`} onClick={() => void previewSound(`custom:${sound.id}`)}>
												<PlayIcon />
												Probar
											</button>
											<button
												type="button"
												className="text-button"
												onClick={() => {
													void removeSound(sound.id)
														.then(() =>
															patch({
																...settings,
																sounds: (settings.sounds ?? []).filter((item) => item.id !== sound.id),
																alarms: settings.alarms.map((item) => (item.sound === `custom:${sound.id}` ? { ...item, sound: "glass" } : item)),
																breaks: settings.breaks.map((item) => (item.sound === `custom:${sound.id}` ? { ...item, sound: "ping" } : item)),
															}),
														)
														.catch((reason: unknown) => setError(reason instanceof Error ? reason.message : "No pude quitar el sonido"));
												}}
											>
												Quitar
											</button>
										</span>
									</li>
								))}
							</ul>
						</div>
						<button type="button" className="text-button" onClick={() => void quitApp()}>
							Salir
						</button>
					</section>
				) : null}

				{error ? <p className="warn">{error}</p> : null}
				</div>
			</section>
		</div>
	);
}
