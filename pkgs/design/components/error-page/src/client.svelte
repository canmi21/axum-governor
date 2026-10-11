<script lang="ts">
	/**
	 * The page for an error the browser met after the page was served: no number, since nothing
	 * answered, only what happened and the way out, centered. See lib's
	 * spec/design/components.md, "An error page".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { Snippet } from 'svelte';
	import { fontSize, lineHeight } from '../../../scale/src/scale.stylex.js';

	let {
		message = 'This page crashed in the browser. Describing what you were doing helps',
		offer,
	}: { message?: string; offer: Snippet } = $props();

	const styles = stylex.create({
		page: {
			display: 'flex',
			minHeight: 'calc(100dvh - 10rem)',
			alignItems: 'center',
			justifyContent: 'center',
		},
		column: {
			display: 'flex',
			flexDirection: 'column',
			alignItems: 'center',
			gap: '1rem',
			textAlign: 'center',
		},
		said: {
			fontSize: fontSize.body,
			lineHeight: lineHeight.relaxed,
			color: 'var(--foreground-default)',
		},
		offer: {
			fontSize: fontSize.label,
			lineHeight: lineHeight.relaxed,
			color: 'var(--foreground-muted)',
		},
	});
</script>

<div class={stylex.attrs(styles.page).class}>
	<div class={stylex.attrs(styles.column).class}>
		<h1 class={stylex.attrs(styles.said).class}>{message}</h1>
		<p class={stylex.attrs(styles.offer).class}>{@render offer()}</p>
	</div>
</div>
