import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			fallback: '200.html'
		}),
		alias: {
			$shadcn: 'src/lib/components/ui',
			$ts: 'src/lib/ts'
		}
	}
};

export default config;