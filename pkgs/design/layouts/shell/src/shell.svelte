<script lang="ts">
	/**
	 * A page's whole frame: the sidebar down the left, its head as tall as the top bar, its pages
	 * below and its foot at the bottom; the top bar across the rest, its three places left, middle
	 * and right; and the page between them, the one region that scrolls. Where the viewport is too
	 * narrow for a sidebar, the three stack into one column that scrolls whole. An app fills each
	 * place and decides nothing of where they stand. See lib's spec/design/layouts.md, "The shell".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { Snippet } from 'svelte';
	import { borderWidth } from '../../../scale/src/scale.stylex.js';

	let {
		label,
		head,
		nav,
		foot,
		start,
		center,
		end,
		children,
		main = $bindable(),
		onscroll,
	}: {
		/** The sidebar's name, for assistive technology. */
		label: string;
		/** The sidebar's head, as tall as the top bar. */
		head: Snippet;
		/** The sidebar's pages, which scroll on their own where they outgrow it. */
		nav: Snippet;
		/** What stands under the pages, at the sidebar's foot. */
		foot?: Snippet;
		/** The top bar's left place. */
		start?: Snippet;
		/** The top bar's middle, centered whatever the sides hold. */
		center?: Snippet;
		/** The top bar's right place. */
		end?: Snippet;
		/** The page. */
		children: Snippet;
		/** The region that scrolls, for an app that keeps its place. */
		main?: HTMLElement;
		onscroll?: (event: Event) => void;
	} = $props();

	const styles = stylex.create({
		sidebar: {
			display: 'flex',
			flexDirection: 'column',
			userSelect: 'none',
			backgroundColor: 'var(--background-base)',
			borderStyle: 'solid',
			borderColor: 'var(--border-subtle)',
			borderTopWidth: 0,
			borderLeftWidth: 0,
			borderBottomWidth: { default: borderWidth.hairline, '@media (min-width: 48rem)': 0 },
			borderRightWidth: { default: 0, '@media (min-width: 48rem)': borderWidth.hairline },
			position: { default: 'static', '@media (min-width: 48rem)': 'fixed' },
			top: { default: null, '@media (min-width: 48rem)': 0 },
			bottom: { default: null, '@media (min-width: 48rem)': 0 },
			left: { default: null, '@media (min-width: 48rem)': 0 },
			zIndex: { default: null, '@media (min-width: 48rem)': 30 },
			width: { default: null, '@media (min-width: 48rem)': '15rem' },
		},
		head: {
			display: 'flex',
			height: '3.5rem',
			flexShrink: 0,
			alignItems: 'center',
			paddingInline: '0.75rem',
		},
		nav: {
			position: 'relative',
			flexGrow: 1,
			flexShrink: 1,
			flexBasis: '0%',
			overflowX: 'hidden',
			overflowY: 'auto',
			paddingInline: '0.75rem',
			paddingTop: '0.25rem',
			paddingBottom: '0.75rem',
		},
		bar: {
			display: 'grid',
			height: '3.5rem',
			gridTemplateColumns: '1fr auto 1fr',
			alignItems: 'center',
			gap: '1rem',
			paddingInline: '2rem',
			userSelect: 'none',
			backgroundColor: 'var(--background-base)',
			borderBottomStyle: 'solid',
			borderBottomWidth: borderWidth.hairline,
			borderBottomColor: 'var(--border-subtle)',
			position: { default: 'static', '@media (min-width: 48rem)': 'fixed' },
			top: { default: null, '@media (min-width: 48rem)': 0 },
			right: { default: null, '@media (min-width: 48rem)': 0 },
			left: { default: null, '@media (min-width: 48rem)': '15rem' },
			zIndex: { default: null, '@media (min-width: 48rem)': 30 },
		},
		start: { display: 'flex', minWidth: 0, alignItems: 'center', justifySelf: 'start' },
		center: { display: 'flex', minWidth: 0, alignItems: 'center', justifySelf: 'center' },
		end: {
			display: 'flex',
			minWidth: 0,
			alignItems: 'center',
			gap: '0.5rem',
			justifySelf: 'end',
		},
		main: {
			position: { default: 'static', '@media (min-width: 48rem)': 'fixed' },
			top: { default: null, '@media (min-width: 48rem)': '3.5rem' },
			right: { default: null, '@media (min-width: 48rem)': 0 },
			bottom: { default: null, '@media (min-width: 48rem)': 0 },
			left: { default: null, '@media (min-width: 48rem)': '15rem' },
			overflowY: { default: null, '@media (min-width: 48rem)': 'auto' },
			overscrollBehavior: { default: null, '@media (min-width: 48rem)': 'contain' },
		},
		page: {
			display: 'flex',
			width: '100%',
			maxWidth: '90rem',
			flexDirection: 'column',
			gap: '1.5rem',
			marginInline: 'auto',
			paddingInline: '2rem',
			paddingTop: '2rem',
			paddingBottom: '3rem',
		},
	});
</script>

<aside aria-label={label} class={stylex.attrs(styles.sidebar).class}>
	<div class={stylex.attrs(styles.head).class}>{@render head()}</div>
	<div class={stylex.attrs(styles.nav).class}>{@render nav()}</div>
	{@render foot?.()}
</aside>
<header class={stylex.attrs(styles.bar).class}>
	<div class={stylex.attrs(styles.start).class}>{@render start?.()}</div>
	<div class={stylex.attrs(styles.center).class}>{@render center?.()}</div>
	<div class={stylex.attrs(styles.end).class}>{@render end?.()}</div>
</header>
<main
	id="content"
	tabindex="-1"
	bind:this={main}
	{onscroll}
	class={stylex.attrs(styles.main).class}
>
	<div class={stylex.attrs(styles.page).class}>{@render children()}</div>
</main>
