
// this file is generated — do not edit it


declare module "svelte/elements" {
	export interface HTMLAttributes<T> {
		'data-sveltekit-keepfocus'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-noscroll'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-preload-code'?:
			| true
			| ''
			| 'eager'
			| 'viewport'
			| 'hover'
			| 'tap'
			| 'off'
			| undefined
			| null;
		'data-sveltekit-preload-data'?: true | '' | 'hover' | 'tap' | 'off' | undefined | null;
		'data-sveltekit-reload'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-replacestate'?: true | '' | 'off' | undefined | null;
	}
}

export {};


declare module "$app/types" {
	export interface AppTypes {
		RouteId(): "/";
		RouteParams(): {
			
		};
		LayoutParams(): {
			"/": Record<string, never>
		};
		Pathname(): "/";
		ResolvedPathname(): `${"" | `/${string}`}${ReturnType<AppTypes['Pathname']>}`;
		Asset(): "/favicon.svg" | "/fonts/chivo-mono-v11-latin-100.woff2" | "/fonts/chivo-mono-v11-latin-100italic.woff2" | "/fonts/chivo-mono-v11-latin-200.woff2" | "/fonts/chivo-mono-v11-latin-200italic.woff2" | "/fonts/chivo-mono-v11-latin-300.woff2" | "/fonts/chivo-mono-v11-latin-300italic.woff2" | "/fonts/chivo-mono-v11-latin-500.woff2" | "/fonts/chivo-mono-v11-latin-500italic.woff2" | "/fonts/chivo-mono-v11-latin-600.woff2" | "/fonts/chivo-mono-v11-latin-600italic.woff2" | "/fonts/chivo-mono-v11-latin-700.woff2" | "/fonts/chivo-mono-v11-latin-700italic.woff2" | "/fonts/chivo-mono-v11-latin-800.woff2" | "/fonts/chivo-mono-v11-latin-800italic.woff2" | "/fonts/chivo-mono-v11-latin-900.woff2" | "/fonts/chivo-mono-v11-latin-900italic.woff2" | "/fonts/chivo-mono-v11-latin-italic.woff2" | "/fonts/chivo-mono-v11-latin-regular.woff2" | "/robots.txt" | string & {};
	}
}