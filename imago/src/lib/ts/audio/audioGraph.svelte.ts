import { eq, EQ_BANDS } from "$ts/store/eqStore.svelte";

export let audioCtx: AudioContext | null = null;
export let sourceNode: MediaElementAudioSourceNode | null = null;
export let filterNodes: BiquadFilterNode[] = [];
export let gainNode: GainNode | null = null;

export function buildAudioGraph(el: HTMLAudioElement) {
	if (!audioCtx) {
		if (typeof window !== 'undefined') {
			audioCtx = new window.AudioContext();
		}
	}

	if (sourceNode) {
		sourceNode.disconnect();
		sourceNode = null;
	}
	filterNodes = [];

	sourceNode = audioCtx!.createMediaElementSource(el);

	const filters = EQ_BANDS.map((freq, i) => {
		const filter = audioCtx!.createBiquadFilter();
		filter.type = i === 0 ? "lowshelf" : i === EQ_BANDS.length - 1 ? "highshelf" : "peaking";
		filter.frequency.value = freq;
		filter.gain.value = eq.enabled ? eq.gains[i] : 0;
		filter.Q.value = 1.0;
		return filter;
	});

	gainNode = audioCtx!.createGain();
	gainNode.gain.value = 1;

	sourceNode.connect(filters[0]);
	for (let i = 0; i < filters.length - 1; i++) {
		filters[i].connect(filters[i + 1]);
	}
	filters[filters.length - 1].connect(gainNode);
	gainNode.connect(audioCtx!.destination);

	filterNodes = filters;
}

export function applyEqToGraph() {
	filterNodes.forEach((filter, i) => {
		filter.gain.value = eq.enabled ? eq.gains[i] : 0;
	});
}
