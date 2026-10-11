<script lang="ts">
	/**
	 * An icon in layers: its box, whose center is the one thing layout aligns to; its drawing's
	 * offset from that center and its scale about it, from ./optics.ts or `optics`, moving only the
	 * drawing through its `viewBox`; and, given the `words` it stands by, the box set in their line,
	 * centered on the middle of the letter nearest it: half an `ex` over the baseline by a lowercase,
	 * half the capital height, `--font-sans-cap-height`, by a capital or a figure. See lib's
	 * spec/design/components.md, "An icon is drawn in layers".
	 */
	import * as stylex from '@stylexjs/stylex';
	import { radius } from '../../../scale/src/scale.stylex.js';
	import { OPTICS, type IconComponent, type Optics, heightBeside, viewBoxOf } from './optics.js';

	let {
		icon: Drawing,
		size,
		stroke = 2,
		optics,
		words,
		side = 'before',
		badge,
		class: className,
	}: {
		icon: IconComponent;
		/** The box's side in pixels. */
		size: number;
		/** The stroke as the drawing is meant to look, before any scale. */
		stroke?: number;
		/** Overrides the icon's row in the table. */
		optics?: Optics;
		/**
		 * The words it stands by in their line, which set it there: its box's center on the middle
		 * of the letter nearest it, wherever its drawing sits inside the box.
		 */
		words?: string;
		/** Which side of `words` it stands on: before them, by their first letter, or after. */
		side?: 'before' | 'after';
		/**
		 * A dot at the box's lower right that says how the thing drawn is, as a class that colors
		 * it; ringed in the ground so it reads as set into the drawing's corner.
		 */
		badge?: string;
		class?: string;
	} = $props();

	const correction = $derived(optics ?? OPTICS.get(Drawing) ?? {});
	const height = $derived(words === undefined ? undefined : heightBeside(words, side));
	/** The box's foot raised so its center meets half that letter's height over the baseline. */
	const raised = $derived(height === undefined ? undefined : `calc(${height} / 2 - ${size / 2}px)`);
	const scale = $derived(correction.scale ?? 1);

	const styles = stylex.create({
		inline: { display: 'inline-block' },
		block: { display: 'block' },
		box: { position: 'relative', flexShrink: 0 },
		/** The dot, ringed in `--badge-ground` where its ground is not a surface. */
		badge: {
			position: 'absolute',
			right: '-1px',
			bottom: '-1px',
			width: '0.375rem',
			height: '0.375rem',
			borderRadius: radius.full,
			boxShadow: '0 0 0 1.5px var(--badge-ground, var(--background-surface))',
		},
	});

	/** StyleX's class for `styles`, then the caller's after it. */
	const joined = (own: string | undefined, theirs: string | undefined) =>
		[own, theirs].filter(Boolean).join(' ');
</script>

{#snippet drawn(placed: boolean)}
	<Drawing
		{size}
		stroke={stroke / scale}
		viewBox={viewBoxOf(correction)}
		aria-hidden="true"
		class={joined(
			stylex.attrs(placed && raised !== undefined && styles.inline, !placed && styles.block).class,
			placed ? className : undefined,
		)}
		style={placed && raised ? `vertical-align: ${raised}` : undefined}
	/>
{/snippet}

{#if badge}
	<!-- The box and its dot as one, which is what the words' line sets. -->
	<span
		class={joined(
			stylex.attrs(styles.box, raised !== undefined ? styles.inline : styles.block).class,
			className,
		)}
		style:width="{size}px"
		style:height="{size}px"
		style:vertical-align={raised}
	>
		{@render drawn(false)}
		<span aria-hidden="true" class={joined(stylex.attrs(styles.badge).class, badge)}></span>
	</span>
{:else}
	{@render drawn(true)}
{/if}
