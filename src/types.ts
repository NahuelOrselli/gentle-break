export interface Alarm {
	id: string;
	name: string;
	time: string;
	days: number[];
	enabled: boolean;
	sound: string;
}

export interface Break {
	id: string;
	name: string;
	message: string;
	everyMinutes: number;
	enabled: boolean;
	sound: string;
}

export interface CustomSound {
	id: string;
	name: string;
	file: string;
}

export interface BreakWindow {
	start: string;
	end: string;
	days: number[];
}

export type WidgetSize = "large" | "medium" | "small";
export type Reaction = "none" | "mustache" | "abuela" | "shake" | "bounce" | "heartbeat";

export type FaceReaction = "mustache" | "abuela";

export function isFaceReaction(reaction: Reaction): reaction is FaceReaction {
	return reaction === "mustache" || reaction === "abuela";
}

export function hasVisualReaction(reaction: Reaction): boolean {
	return reaction !== "none";
}

export interface Settings {
	dnd: boolean;
	widgetVisible: boolean;
	widgetSize: WidgetSize;
	reaction: Reaction;
	breakWindow: BreakWindow;
	alarms: Alarm[];
	breaks: Break[];
	sounds: CustomSound[];
}

export const PRESET_SOUNDS: { id: string; label: string }[] = [
	{ id: "mustache", label: "Mostacho" },
	{ id: "abuela", label: "Abuela" },
	{ id: "glass", label: "Cristal" },
	{ id: "ping", label: "Ping" },
	{ id: "pop", label: "Pop" },
	{ id: "blow", label: "Soplo" },
	{ id: "funk", label: "Funk" },
	{ id: "hero", label: "Hero" },
	{ id: "tink", label: "Tintineo" },
	{ id: "submarine", label: "Submarino" },
	{ id: "random", label: "Aleatorio" },
];

export const REACTIONS: { id: Reaction; label: string }[] = [
	{ id: "none", label: "Nada" },
	{ id: "mustache", label: "Mostacho" },
	{ id: "abuela", label: "Abuela" },
	{ id: "shake", label: "Sacudida" },
	{ id: "bounce", label: "Saltos" },
	{ id: "heartbeat", label: "Latido" },
];

export const WIDGET_SIZES: { id: WidgetSize; label: string }[] = [
	{ id: "large", label: "Grande" },
	{ id: "medium", label: "Mediano" },
	{ id: "small", label: "Chico" },
];

export function clockMetrics(size: WidgetSize): { pixel: number; gap: number; letterGap: number } {
	if (size === "small") return { pixel: 2, gap: 0, letterGap: 1 };
	if (size === "medium") return { pixel: 2, gap: 1, letterGap: 2 };
	return { pixel: 3, gap: 1, letterGap: 3 };
}

export const WEEKDAYS: { id: number; label: string; name: string }[] = [
	{ id: 1, label: "L", name: "lunes" },
	{ id: 2, label: "M", name: "martes" },
	{ id: 3, label: "X", name: "miércoles" },
	{ id: 4, label: "J", name: "jueves" },
	{ id: 5, label: "V", name: "viernes" },
	{ id: 6, label: "S", name: "sábado" },
	{ id: 7, label: "D", name: "domingo" },
];

export const DEFAULT_SETTINGS: Settings = {
	dnd: false,
	widgetVisible: true,
	widgetSize: "large",
	reaction: "mustache",
	breakWindow: { start: "09:00", end: "18:00", days: [1, 2, 3, 4, 5] },
	alarms: [
		{ id: "almuerzo", name: "Almuerzo", time: "13:00", days: [1, 2, 3, 4, 5], enabled: true, sound: "glass" },
		{ id: "cierre", name: "Cierre del día", time: "18:00", days: [1, 2, 3, 4, 5], enabled: true, sound: "hero" },
	],
	breaks: [
		{
			id: "caminar",
			name: "Caminar",
			message: "Parate a caminar",
			everyMinutes: 120,
			enabled: true,
			sound: "ping",
		},
	],
	sounds: [],
};
