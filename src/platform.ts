import { DEFAULT_SETTINGS, type CustomSound, type Settings } from "./types.ts";

const STORAGE_KEY = "gentle-break-settings";

export function isTauri(): boolean {
	return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getState(): Promise<Settings> {
	if (!isTauri()) {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return structuredClone(DEFAULT_SETTINGS);
		try {
			const parsed = JSON.parse(raw) as Partial<Settings>;
			return {
				...structuredClone(DEFAULT_SETTINGS),
				...parsed,
				alarms: (parsed.alarms ?? DEFAULT_SETTINGS.alarms).map((alarm) => ({ ...alarm, sound: alarm.sound || "glass" })),
				breaks: (parsed.breaks ?? DEFAULT_SETTINGS.breaks).map((item) => ({ ...item, sound: item.sound || "ping" })),
				sounds: parsed.sounds ?? [],
			};
		} catch {
			return structuredClone(DEFAULT_SETTINGS);
		}
	}
	const { invoke } = await import("@tauri-apps/api/core");
	return invoke<Settings>("get_state");
}

export async function saveState(settings: Settings): Promise<void> {
	if (!isTauri()) {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
		window.dispatchEvent(
			new CustomEvent("gentle-break-live", {
				detail: {
					dnd: settings.dnd,
					widgetVisible: settings.widgetVisible,
					widgetSize: settings.widgetSize,
					reaction: settings.reaction,
				},
			}),
		);
		return;
	}
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("save_state", { settings });
}

export async function togglePanel(): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("toggle_panel");
}

export async function hidePanel(): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("hide_panel");
}

export async function chooseSoundFile(): Promise<string | null> {
	if (!isTauri()) return null;
	const { open } = await import("@tauri-apps/plugin-dialog");
	const picked = await open({
		multiple: false,
		title: "Elegir sonido",
		filters: [{ name: "Audio", extensions: ["wav", "mp3", "m4a", "aiff", "aif", "caf"] }],
	});
	return typeof picked === "string" ? picked : null;
}

export async function importSound(path: string): Promise<CustomSound> {
	const { invoke } = await import("@tauri-apps/api/core");
	return invoke<CustomSound>("import_sound", { path });
}

export async function previewReaction(reaction: string, title: string, body: string): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("preview_reaction", { reaction, title, body });
}

export async function previewSound(sound: string): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("preview_sound", { sound });
}

export async function removeSound(id: string): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("remove_sound", { id });
}

export async function quitApp(): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import("@tauri-apps/api/core");
	await invoke("quit");
}

export async function startDragging(): Promise<void> {
	if (!isTauri()) return;
	const { getCurrentWindow } = await import("@tauri-apps/api/window");
	await getCurrentWindow().startDragging();
}

export function pad(value: number): string {
	return String(value).padStart(2, "0");
}
