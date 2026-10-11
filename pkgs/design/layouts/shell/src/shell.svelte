<script lang="ts">
	/**
	 * A page's whole frame: the sidebar down the left, its head as tall as the top bar, its pages
	 * below and its foot at the bottom; the top bar, its three places left, middle and right; and the
	 * page between them, the one region that scrolls. `span` says which of the two bars runs the
	 * whole edge -- the sidebar from top to bottom with the top bar cut at it, or the top bar from
	 * side to side, one band, with the sidebar under it and `corner` in the bar's place over it -- and
	 * `density` how tall the top bar stands. Where the viewport is too narrow
	 * for a sidebar, the regions stack into one column that scrolls whole. An app fills each place
	 * and decides nothing of where they stand. See lib's spec/design/layouts.md, "The shell".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { Snippet } from 'svelte';
	import { borderWidth } from '../../../scale/src/scale.stylex.js';

	let {
		label,
		density = 'regular',
		span = 'sidebar',
		corner,
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
		/** The top bar's height: `regular` 4rem, `compact` 3.5rem. */
		density?: 'regular' | 'compact';
		/** Which bar runs the whole edge: the sidebar top to bottom, or the top bar side to side. */
		span?: 'sidebar' | 'bar';
		/** The bar's corner over the sidebar, where the bar spans: a name, a mark. */
		corner?: Snippet;
		/** The sidebar's head, as tall as the top bar, under the bar where the bar spans. */
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
			gridTemplateColumns: '1fr auto 1fr',
			alignItems: 'center',
			columnGap: '1rem',
			rowGap: 0,
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
		/**
		 * The bar across the whole width, one band with nothing cutting it: its first place an empty
		 * corner as wide as the sidebar, so the bar's sides stand over the page as they would were it
		 * cut; the sidebar's edge starts under the bar.
		 */
		barSpans: {
			gridTemplateColumns: {
				default: '1fr auto 1fr',
				'@media (min-width: 48rem)': '15rem 1fr auto 1fr',
			},
			paddingInline: { default: '2rem', '@media (min-width: 48rem)': 0 },
			left: { default: null, '@media (min-width: 48rem)': 0 },
		},
		/**
		 * The corner, inset to the column the sidebar's icons stand on; where there is no sidebar
		 * beside the page, not there either.
		 */
		corner: {
			display: { default: 'none', '@media (min-width: 48rem)': 'flex' },
			alignItems: 'center',
			alignSelf: 'stretch',
			paddingInline: '1.5rem',
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
		/**
		 * The bar's sides inset where the bar spans, so each stands where it would were the bar cut:
		 * the left a rem past the grid's own gap after the head, 2rem in all, over the page's column.
		 */
		startInset: { paddingLeft: { default: 0, '@media (min-width: 48rem)': '1rem' } },
		endInset: { paddingRight: { default: 0, '@media (min-width: 48rem)': '2rem' } },
		main: {
			position: { default: 'static', '@media (min-width: 48rem)': 'fixed' },
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
		/** The top bar's height, and the sidebar's head beside it. */
		tallRegular: { height: '4rem' },
		tallCompact: { height: '3.5rem' },
		/** What stands under the bar starts at its foot. */
		belowRegular: { top: { default: null, '@media (min-width: 48rem)': '4rem' } },
		belowCompact: { top: { default: null, '@media (min-width: 48rem)': '3.5rem' } },
	});

	const regular = $derived(density === 'regular');
	const spans = $derived(span === 'bar');
	const tall = $derived(regular ? styles.tallRegular : styles.tallCompact);
	const below = $derived(regular ? styles.belowRegular : styles.belowCompact);
</script>

{#snippet sidebar()}
	<aside aria-label={label} class={stylex.attrs(styles.sidebar, spans && below).class}>
		<div class={stylex.attrs(styles.head, tall).class}>{@render head()}</div>
		<div class={stylex.attrs(styles.nav).class}>{@render nav()}</div>
		{@render foot?.()}
	</aside>
{/snippet}

{#snippet bar()}
	<header class={stylex.attrs(styles.bar, tall, spans && styles.barSpans).class}>
		{#if spans}
			<div class={stylex.attrs(styles.corner).class}>{@render corner?.()}</div>
		{/if}
		<div class={stylex.attrs(styles.start, spans && styles.startInset).class}>
			{@render start?.()}
		</div>
		<div class={stylex.attrs(styles.center).class}>{@render center?.()}</div>
		<div class={stylex.attrs(styles.end, spans && styles.endInset).class}>{@render end?.()}</div>
	</header>
{/snippet}

<!-- Whichever spans comes first, so a narrow page stacks it on top. -->
{#if spans}
	{@render bar()}
	{@render sidebar()}
{:else}
	{@render sidebar()}
	{@render bar()}
{/if}
<main
	id="content"
	tabindex="-1"
	bind:this={main}
	{onscroll}
	class={stylex.attrs(styles.main, below).class}
>
	<div class={stylex.attrs(styles.page).class}>{@render children()}</div>
</main>
