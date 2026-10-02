<script lang="ts">

	// COMPONENTS
	import { CirclePlay, CirclePause, SkipBack, SkipForward, Pause, Play,
				Shuffle, TrendingUpDown, Repeat, Repeat1,
				Volume, Volume2, VolumeX, Volume1, VolumeOff
	} from "lucide-svelte";
	
	import { Button } from "$shadcn/button";
	import { Slider } from "$shadcn/slider/index.js";

	// SCRIPTS
	import { getQueuedTracks, getQueuedTracksAll, togglePlay, seek, skipBack, skipNext, toggleLoop, toggleShuffle, setVolume, toggleMute } from "$ts/audio/audioManager.svelte";
	import { player, getQueueIndex } from "$ts/audio/audioPlayer.svelte";

	// VARIABLES
	let seeking = $state(false);
	let seekValue = $state(0);

	// APP FUNCTIONS
	$effect(() => {
		if (!seeking) {
			seekValue = player.currentTime;
		}
	});

	// FUNCTIONS
	function onSliderChange(value: number[]) {
		seekValue = value[0];
	}
	function onSliderCommit(value: number[]) {
		seek(value[0]);
		seeking = false;
	}
	function onSliderStart() {
		seeking = true;
	}

	function formatTime(seconds: number): string {
		if (!seconds || isNaN(seconds)) return "0:00";
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		return `${m}:${s.toString().padStart(2, "0")}`;
	}

	let canSkipForward = $derived.by(() => {
		let hasNextTrack = getQueuedTracks().length > 0;
		let canLoopAround = player.loopType === 2 && player.track !== null && getQueuedTracksAll().length > 0;

		return hasNextTrack || canLoopAround;
	})

</script>

<div class="app-playbar bg-muted grid p-2 gap-1 rounded-md">

	<div class="app-playbar-buttons-wrapper grid">
		<span></span>
		
		<div class="flex justify-center items-center gap-2">
			<Button variant="ghost" size="icon" onclick={toggleShuffle} class="cursor-pointer">
				{#if player.shuffleType === 0}
					<Shuffle class="text-muted-foreground hover:text-foreground" />
				{:else if player.shuffleType === 1}
					<Shuffle />
				{:else if player.shuffleType === 2}
					<TrendingUpDown />
				{:else}
					<Shuffle />
				{/if}
			</Button>
			<Button variant="ghost" size="icon" onclick={skipBack} class="cursor-pointer">
				<SkipBack />
			</Button>
			<button
				onclick={togglePlay}
				class="play-btn"
			>
				{#if player.isPlaying}
					<Pause size={20} fill="var(--muted)" color="var(--muted)" />
				{:else}
					<Play size={20} fill="var(--muted)" color="var(--muted)" />
				{/if}
			</button>
			<Button variant="ghost" size="icon" disabled={!canSkipForward} onclick={skipNext} class="cursor-pointer">
				<SkipForward />
			</Button>
			<Button variant="ghost" size="icon" onclick={toggleLoop} class="cursor-pointer">
				{#if player.loopType === 0}
					<Repeat class="text-muted-foreground" />
				{:else if player.loopType === 1}
					<Repeat1 />
				{:else}
					<Repeat />
				{/if}
			</Button>
		</div>

		<div class="flex justify-center items-center">
			<Button variant="ghost" size="icon" onclick={toggleMute} class="cursor-pointer">
				{#if player.volume === 0 || player.muted}
					<VolumeOff class="text-muted-foreground" />
				{:else if player.volume < 0.25}
				<Volume />
				{:else if player.volume < 0.75}
				<Volume1 />
				{:else}
				<Volume2 />
				{/if}
			</Button>
			<Slider
				type="single"
				value={player.volume}
				min={0}
				max={1}
				step={0.01}
				onValueChange={(v) => setVolume(v)}
				onValueCommit={(v) => setVolume(v)}
				class="w-[100px] app-tracking-slider"
			/>
		</div>
	</div>

	<div class="app-bar grid gap-2 items-center">
		<span class="text-xs text-muted-foreground">{formatTime(player.currentTime)}</span>
		
		<div class="flex-1" onpointerdown={onSliderStart} aria-hidden="true" tabindex="-1">
			<Slider
				type="single"
				value={seeking ? seekValue : player.currentTime}
				min={0}
				max={player.duration || 1}
				step={0.1}
				disabled={!player.track}
				onValueChange={(v) => onSliderChange([v])}
				onValueCommit={(v) => onSliderCommit([v])}
				class="w-full app-tracking-slider"
			/>
		</div>

		<span class="text-xs text-muted-foreground">{formatTime(player.duration)}</span>
	</div>

</div>

<style>
.app-playbar {
	max-height: 85px;
	height: 85px;
}

.app-playbar-buttons-wrapper {
	grid-template-columns: 120px 1fr 120px;
}

.app-bar {
	grid-template-columns: auto 1fr auto;
}


:global([data-slider-thumb]) {
	opacity: 0;
}

:global([data-slider-thumb]:hover) {
	opacity: 1;
}

:global(.app-tracking-slider [data-slot="slider-track"]) {
	background-color: oklch(from var(--background) l c h / 50%) !important;
}

:global(.app-tracking-slider [data-slot="slider-range"]) {
	background-color: var(--accent) !important;
}

.play-btn {
	width: 42px;
	height: 42px;
	border-radius: 50%;
	background-color: var(--foreground);
	border: none;
	cursor: pointer;
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0;
	transition: transform 0.1s ease, opacity 0.1s ease;
}

.play-btn:hover {
	opacity: 0.85;
	transform: scale(1.05);
}

.play-btn:active {
	transform: scale(0.97);
}

</style>