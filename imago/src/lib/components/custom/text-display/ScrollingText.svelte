<script lang="ts">
	import { onMount, tick } from 'svelte';

	// PROPS
	let {
		text,
		class: className = '',
		speed = 20,
		pauseMs = 1600,
		hoverOnly = false,
		onclick,
	} = $props<{
		text: string;
		class?: string;
		speed?: number;
		pauseMs?: number;
		hoverOnly?: boolean;
		onclick?: () => void;
	}>();

	// VARIABLES
	let containerEl: HTMLElement;
	let textEl: HTMLElement;
	let animating = $state(false);
	let returning = $state(false);

	let translateX = $state(0);
	let transitioning = $state(false);
	let transitionDuration = $state(0);

	let timeoutId: ReturnType<typeof setTimeout>;
		
	let resizeObserver: ResizeObserver;

	// APP FUNCTIONS
	onMount(() => {
		resizeObserver = new ResizeObserver(() => checkOverflow());
		if (containerEl) resizeObserver.observe(containerEl);
		checkOverflow();

		return () => {
			cleanup();
			resizeObserver?.disconnect();
		};
	});

	$effect(() => {
		text;
		if (containerEl) {
			cleanup();
			timeoutId = setTimeout(() => checkOverflow(), 0);
		}
	});

	// FUNCTIONS
	function getCurrentTranslateX(): number {
		if (!textEl) return 0;
		const style = window.getComputedStyle(textEl);
		const matrix = new DOMMatrix(style.transform);
		return matrix.m41;
	}

	function pause(ms: number): Promise<void> {
		return new Promise((resolve) => {
			timeoutId = setTimeout(resolve, ms);
		});
	}

	function cleanup() {
		clearTimeout(timeoutId);
		animating = false;
		returning = false;
		translateX = 0;
		transitioning = false;
		transitionDuration = 0;
	}

	async function returnToStart() {
		returning = true;
		animating = false;
		clearTimeout(timeoutId);

		// Read the actual current position from the DOM mid-animation
		const currentX = getCurrentTranslateX();

		// Snap state to current visual position without transition
		transitioning = false;
		transitionDuration = 0;
		translateX = currentX;

		await tick();

		// Now animate back to 0 at 2x speed
		const distance = Math.abs(currentX);
		if (distance < 1) {
			returning = false;
			return;
		}

		const returnDuration = (distance / (speed * 2)) * 1000;
		transitionDuration = returnDuration;
		transitioning = true;
		translateX = 0;

		await pause(returnDuration);
		transitioning = false;
		returning = false;
	}

	async function runScrollLoop() {
		if (!containerEl || !textEl) return;

		const containerWidth = containerEl.clientWidth;
		const textWidth = textEl.scrollWidth;
		if (textWidth <= containerWidth) return;

		animating = true;
		const scrollDistance = textWidth - containerWidth;
		const forwardDuration = (scrollDistance / speed) * 1000;
		const returnDuration = (scrollDistance / speed) * 1000;

		while (animating) {
			translateX = 0;
			transitioning = false;
			transitionDuration = 0;

			await tick();
			await pause(pauseMs);
			if (!animating) break;

			transitionDuration = forwardDuration;
			transitioning = true;
			translateX = -scrollDistance;

			await pause(forwardDuration + pauseMs);
			if (!animating) break;

			transitionDuration = returnDuration;
			transitioning = true;
			translateX = 0;

			await pause(returnDuration + pauseMs);
		}
	}

	async function runScrollHover() {
		if (!containerEl || !textEl) return;

		const containerWidth = containerEl.clientWidth;
		const textWidth = textEl.scrollWidth;
		if (textWidth <= containerWidth) return;

		animating = true;
		const scrollDistance = textWidth - containerWidth;
		const forwardDuration = (scrollDistance / speed) * 1000;

		while (animating) {
			translateX = 0;
			transitioning = false;
			transitionDuration = 0;

			await tick();

			// No initial pause — start scrolling immediately on hover
			transitionDuration = forwardDuration;
			transitioning = true;
			translateX = -scrollDistance;

			await pause(forwardDuration + pauseMs);
			if (!animating) break;

			// Pause at end, then scroll back at normal speed before looping
			transitionDuration = forwardDuration;
			transitioning = true;
			translateX = 0;

			await pause(forwardDuration + pauseMs);
		}
	}

	function checkOverflow() {
		cleanup();
		if (!hoverOnly) {
			runScrollLoop();
		}
	}

	async function onMouseEnter() {
		if (!hoverOnly) return;
		if (returning) {
			// Wait for return animation to finish before starting forward scroll
			const interval = setInterval(() => {
				if (!returning) {
					clearInterval(interval);
					runScrollHover();
				}
			}, 16);
			return;
		}
		if (!animating) {
			runScrollHover();
		}
	}

	function onMouseLeave() {
		if (!hoverOnly) return;
		returnToStart();
	}

</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	bind:this={containerEl}
	class="scrolling-text-container {className}"
	role={onclick ? 'button' : undefined}
	tabindex={onclick ? 0 : undefined}
	onclick={onclick}
	onkeydown={onclick ? (e) => e.key === 'Enter' && onclick() : undefined}
	onmouseenter={onMouseEnter}
	onmouseleave={onMouseLeave}
>
	<span
		bind:this={textEl}
		class="scrolling-text-inner"
		style:transform="translateX({translateX}px)"
		style:transition={transitioning ? `transform ${transitionDuration}ms linear` : 'none'}
	>
		{text}
	</span>
</div>

<style>
	.scrolling-text-container {
		overflow: hidden;
		white-space: nowrap;
		width: 100%;
		min-width: 0;
		display: block;
	}

	.scrolling-text-container[role='button'] {
		cursor: pointer;
	}

	.scrolling-text-inner {
		display: inline-block;
		white-space: nowrap;
		will-change: transform;
	}
</style>