<div class="app-music-grid grid gap-3 pr-4 pl-4 pb-4"><slot /></div>

<style>
.app-music-grid {
	grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
}
</style>