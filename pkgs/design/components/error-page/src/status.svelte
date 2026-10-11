<script lang="ts">
	/**
	 * The page for an error a server answered with a number: the number, a hairline, and a
	 * sentence for a person, on one line in the middle of the page, and the way out at its foot.
	 * Which page an error is, and the protocol's names, are @canmi/web/error's. See lib's
	 * spec/design/components.md, "An error page".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { Snippet } from 'svelte';
	import { borderWidth, fontSize, fontWeight } from '../../../scale/src/scale.stylex.js';

	let {
		status,
		name,
		message,
		offer,
	}: {
		status: number;
		/** The protocol's name for the status, which a reader hears after the number. */
		name: string;
		/** The page's own sentence, never the error's message, which is for the log. */
		message?: string;
		/** The way out, at the page's foot. */
		offer: Snippet;
	} = $props();

	const said = $derived(
		message ?? (status === 404 ? 'This page could not be found' : 'Something went wrong'),
	);

	const styles = stylex.create({
		page: {
			position: 'relative',
			display: 'flex',
			minHeight: 'calc(100dvh - 10rem)',
			alignItems: 'center',
			justifyContent: 'center',
		},
		line: { display: 'flex', alignItems: 'center' },
		/** The number, and the rule between it and the sentence. */
		status: {
			paddingRight: '1.5rem',
			borderRightWidth: borderWidth.hairline,
			borderRightStyle: 'solid',
			borderColor: 'var(--border-default)',
			fontSize: '1.5rem', // unnamed: the page's one large thing, written once
			lineHeight: '3.0625rem', // unnamed: one box for the number and the sentence
			fontWeight: fontWeight.medium,
			color: 'var(--foreground-strong)',
		},
		message: {
			paddingLeft: '1.5rem',
			fontSize: fontSize.body,
			lineHeight: '3.0625rem', // unnamed: the number's box, which puts the two on one line
			color: 'var(--foreground-default)',
		},
		offer: {
			position: 'absolute',
			right: 0,
			bottom: '1rem',
			left: 0,
			textAlign: 'center',
			fontSize: fontSize.label,
			color: 'var(--foreground-muted)',
		},
		/** Heard and not seen. */
		heard: {
			position: 'absolute',
			width: '1px',
			height: '1px',
			padding: 0,
			margin: '-1px',
			overflow: 'hidden',
			clip: 'rect(0, 0, 0, 0)',
			whiteSpace: 'nowrap',
			borderWidth: 0,
		},
	});
</script>

<div class={stylex.attrs(styles.page).class}>
	<div class={stylex.attrs(styles.line).class}>
		<h1 class={stylex.attrs(styles.status).class}>
			<!-- The space as data: Svelte trims it as markup, and a reader heard one word. -->
			{status}<span class={stylex.attrs(styles.heard).class}>{` ${name}`}</span>
		</h1>
		<p class={stylex.attrs(styles.message).class}>{said}</p>
	</div>
	<p class={stylex.attrs(styles.offer).class}>{@render offer()}</p>
</div>
