<script lang="ts">
	/**
	 * A part of a page held in one theme whatever the page's: every token inside takes that theme's
	 * value. Bare it lays out nothing of its own, so it may hold as little as one swatch; with
	 * `ground` it is a block, a window onto that theme painted in its base and text. See lib's
	 * spec/design/components.md, "A theme scope".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		theme,
		ground = false,
		children,
		...rest
	}: {
		theme: 'light' | 'dark';
		/** Paint the theme's base and text, for a scope that is a window rather than a swatch. */
		ground?: boolean;
		children?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'class'> = $props();

	const styles = stylex.create({
		bare: { display: 'contents' },
		ground: {
			backgroundColor: 'var(--background-base)',
			color: 'var(--foreground-default)',
		},
	});
</script>

<div {...rest} data-theme={theme} class={stylex.attrs(ground ? styles.ground : styles.bare).class}>
	{@render children?.()}
</div>
