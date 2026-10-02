<script lang="ts">
	import { Switch } from "$shadcn/switch/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { eq, EQ_BANDS, EQ_PRESETS, setEqEnabled, setEqBandGain, applyEqPreset } from "$ts/store/eqStore.svelte";
	import { applyEqToGraph } from "$ts/audio/audioGraph.svelte";

	async function handleEqToggle(checked: boolean) {
		await setEqEnabled(checked);
		applyEqToGraph();
	}

	async function handleBandChange(index: number, value: number) {
		await setEqBandGain(index, value);
		applyEqToGraph();
	}

	async function handlePreset(preset: (typeof EQ_PRESETS)[number]) {
		await applyEqPreset(preset);
		applyEqToGraph();
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-3 bg-muted rounded-md">
		<div class="flex items-center justify-between">
			<div>
				<p class="text-sm font-semibold">Equalizer</p>
				<p class="text-xs text-muted-foreground">10-band graphic EQ applied to audio output.</p>
			</div>
			<Switch checked={eq.enabled} onCheckedChange={handleEqToggle} />
		</div>

		<div class="flex flex-wrap gap-2">
			{#each EQ_PRESETS as preset}
				<Button
					variant="outline"
					class="text-xs h-7 px-2"
					onclick={() => handlePreset(preset)}
				>{preset.name}</Button>
			{/each}
		</div>

		<div
			class="flex items-end justify-between gap-1 pt-2"
			class:opacity-50={!eq.enabled}
			class:pointer-events-none={!eq.enabled}
		>
			{#each EQ_BANDS as freq, i}
				<div class="flex flex-col items-center gap-1 flex-1">
					<span class="text-xs text-muted-foreground tabular-nums">
						{eq.gains[i] > 0 ? "+" : ""}{eq.gains[i]}
					</span>
					<div class="relative flex justify-center" style="height: 120px;">
						<input
							type="range"
							min="-12"
							max="12"
							step="0.5"
							value={eq.gains[i]}
							oninput={(e) => handleBandChange(i, parseFloat((e.target as HTMLInputElement).value))}
							style="writing-mode: vertical-lr; direction: rtl; width: 28px; height: 120px; cursor: pointer; accent-color: hsl(var(--primary));"
						/>
					</div>
					<span class="text-xs text-muted-foreground">
						{freq >= 1000 ? `${freq / 1000}k` : freq}
					</span>
				</div>
			{/each}
		</div>

		<div class="flex justify-between text-xs text-muted-foreground px-1 mt-1">
			<span>-12 dB</span>
			<span>0 dB</span>
			<span>+12 dB</span>
		</div>
	</div>
</div>
