<script lang="ts">
	/**
	 * The ways out of an error page, as one sentence: a report, where the app can take one, and an
	 * address to write to. Each told from the muted sentence around it by its ink alone. The app
	 * gives both; see lib's spec/design/components.md, "An error page".
	 */
	import * as stylex from '@stylexjs/stylex';
	import { duration } from '../../../scale/src/scale.stylex.js';

	let {
		address,
		report,
		missing = false,
	}: {
		/** Where to write. */
		address: string;
		/** Opens the app's report form; none, and only the address is offered. */
		report?: () => unknown;
		/** An absence rather than a failure: nothing to report, so only the address. */
		missing?: boolean;
	} = $props();

	const styles = stylex.create({
		control: {
			color: {
				default: 'var(--foreground-default)',
				':hover': 'var(--foreground-strong)',
			},
			transitionProperty: 'color',
			transitionDuration: duration.base,
		},
		button: {
			padding: 0,
			borderWidth: 0,
			backgroundColor: 'transparent',
			font: 'inherit',
			cursor: 'pointer',
		},
	});
</script>

{#snippet written()}<a href="mailto:{address}" class={stylex.attrs(styles.control).class}
		>{address}</a
	>{/snippet}

{#if missing || !report}
	If you think this is a mistake, write to {@render written()}
{:else}
	File a <button
		type="button"
		class={stylex.attrs(styles.button, styles.control).class}
		onclick={() => void report()}>report</button
	>
	or email {@render written()}
{/if}
