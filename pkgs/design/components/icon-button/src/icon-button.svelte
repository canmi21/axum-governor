<script lang="ts">
	/**
	 * A button that is only an icon, its `label` its name and its hover. `ghost` has no ground of its
	 * own and rises on hover, for a bar's own controls; `framed` is ruled and filled, for a control
	 * set apart from what is around it. Either is square or round, and with an `href` it is a link.
	 * See lib's spec/design/components.md, "An icon button".
	 */
	import * as stylex from '@stylexjs/stylex';
	import { borderWidth, duration, radius } from '../../../scale/src/scale.stylex.js';
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		label,
		variant = 'ghost',
		size = 'md',
		shape = 'square',
		href,
		external = false,
		type = 'button',
		children,
		...rest
	}: {
		label: string;
		variant?: 'ghost' | 'framed';
		/** `sm` is 1.75rem square, `md` 2rem. */
		size?: 'sm' | 'md';
		/** `square` has the small radius; `circle` is round. */
		shape?: 'square' | 'circle';
		href?: string;
		/** A link that leaves the app, opened beside it. */
		external?: boolean;
		children: Snippet;
	} & HTMLButtonAttributes = $props();

	const styles = stylex.create({
		base: {
			display: 'inline-flex',
			flexShrink: 0,
			alignItems: 'center',
			justifyContent: 'center',
			cursor: 'pointer',
			borderWidth: 0,
			transitionProperty: 'color, background-color, box-shadow',
			transitionDuration: duration.base,
		},
		square: { borderRadius: radius.md },
		circle: { borderRadius: radius.full },
		sm: { width: '1.75rem', height: '1.75rem' },
		md: { width: '2rem', height: '2rem' },
		ghost: {
			backgroundColor: {
				default: 'transparent',
				':hover': 'var(--interaction-default)',
			},
			color: { default: 'var(--foreground-muted)', ':hover': 'var(--foreground-strong)' },
		},
		// Ruled by a shadow rather than a border, so the rule takes no room and the button is exactly
		// its size; the hover lays the strong rank over the fill and leaves the rule.
		framed: {
			backgroundColor: {
				default: 'var(--background-surface)',
				':hover': 'var(--interaction-strong)',
			},
			boxShadow: `0 0 0 ${borderWidth.hairline} var(--border-default)`,
			color: { default: 'var(--foreground-muted)', ':hover': 'var(--foreground-strong)' },
		},
	});
	const kind = $derived(
		stylex.attrs(styles.base, styles[shape], styles[size], styles[variant]).class,
	);
</script>

{#if href}
	<a
		{href}
		target={external ? '_blank' : undefined}
		rel={external ? 'noopener' : undefined}
		aria-label={label}
		title={label}
		class={kind}>{@render children()}</a
	>
{:else}
	<button {type} {...rest} aria-label={label} title={label} class={kind}
		>{@render children()}</button
	>
{/if}
