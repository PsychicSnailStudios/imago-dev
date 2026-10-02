import { invoke } from "@tauri-apps/api/core";

export const EQ_BANDS = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const;
export type EqBands = typeof EQ_BANDS;

export type EqGains = [number, number, number, number, number, number, number, number, number, number];

export type EqPreset = {
	name: string;
	gains: EqGains;
};

export const EQ_PRESETS: EqPreset[] = [
	{
		name: "Flat",
		gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
	},
	{
		name: "Bass Boost",
		gains: [8, 7, 6, 4, 1, 0, 0, 0, 0, 0],
	},
	{
		name: "Treble Boost",
		gains: [0, 0, 0, 0, 0, 1, 3, 5, 7, 8],
	},
	{
		name: "Vocal",
		gains: [-2, -1, 0, 3, 5, 5, 4, 2, 1, 0],
	},
	{
		name: "Electronic",
		gains: [6, 5, 2, 0, -2, 0, 2, 4, 5, 6],
	},
	{
		name: "Rock",
		gains: [5, 4, 2, 0, -1, 0, 2, 4, 5, 5],
	},
	{
		name: "Jazz",
		gains: [3, 2, 1, 3, 0, 0, -1, -1, 2, 3],
	},
	{
		name: "Classical",
		gains: [4, 3, 2, 1, 0, 0, -1, -2, -2, 0],
	},
];

const SETTING_KEY_ENABLED = "eq_enabled";
const SETTING_KEY_GAINS = "eq_gains";

function defaultGains(): EqGains {
	return [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
}

export const eq = $state({
	enabled: false,
	gains: defaultGains() as EqGains,
});

export async function loadEqSettings() {
	try {
		const raw: { key: string; value: string }[] = await invoke("get_settings");
		const map = Object.fromEntries(raw.map((s) => [s.key, s.value]));

		if (map[SETTING_KEY_ENABLED] !== undefined) {
			eq.enabled = map[SETTING_KEY_ENABLED] === "true";
		}
		if (map[SETTING_KEY_GAINS]) {
			const parsed = JSON.parse(map[SETTING_KEY_GAINS]);
			if (Array.isArray(parsed) && parsed.length === 10) {
				eq.gains = parsed as EqGains;
			}
		}
	} catch {}
}

async function persistEnabled() {
	await invoke("save_setting", { key: SETTING_KEY_ENABLED, value: String(eq.enabled) });
}

async function persistGains() {
	await invoke("save_setting", { key: SETTING_KEY_GAINS, value: JSON.stringify(eq.gains) });
}

export async function setEqEnabled(enabled: boolean) {
	eq.enabled = enabled;
	await persistEnabled();
}

export async function setEqBandGain(index: number, gain: number) {
	eq.gains[index] = gain;
	await persistGains();
}

export async function applyEqPreset(preset: EqPreset) {
	eq.gains = [...preset.gains] as EqGains;
	await persistGains();
}
