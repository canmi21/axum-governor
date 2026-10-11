<script lang="ts">
	/**
	 * One page in a list of pages: its icon and name, the pointer's tint over it, and the
	 * selection's when it is the page being read or a page under it. Its `current` is what
	 * assistive technology is told: `page` where it is the page, `true` where the page is somewhere
	 * under it. See lib's spec/design/components.md, "A link to a page".
	 */
	import * as stylex from '@stylexjs/stylex';
	import type { HTMLAnchorAttributes } from 'svelte/elements';
	import { duration, fontSize, fontWeight, radius } from '../../../scale/src/scale.stylex.js';
	import Icon from '../../icon/src/icon.svelte';
	import type { IconComponent } from '../../icon/src/optics.js';

	let {
		href,
		label,
		icon,
		current = false,
		...rest
	}: {
		href: string;
		label: string;
		icon: IconComponent;
		current?: 'page' | 'true' | false;
	} & Omit<HTMLAnchorAttributes, 'href' | 'class'> = $props();

	const styles = stylex.create({
		link: {
			display: 'flex',
			height: '2.25rem',
			alignItems: 'center',
			gap: '0.625rem',
			paddingInline: '0.75rem',
			borderRadius: radius.md,
			backgroundColor: { default: 'transparent', ':hover': 'var(--interaction-hover)' },
			color: { default: 'var(--foreground-muted)', ':hover': 'var(--foreground-strong)' },
			fontSize: fontSize.body,
			fontWeight: fontWeight.medium,
			transitionProperty: 'color, background-color',
			transitionDuration: duration.base,
		},
		current: {
			backgroundColor: {
				default: 'var(--interaction-selected)',
				':hover': 'var(--interaction-selected)',
			},
			color: { default: 'var(--foreground-strong)', ':hover': 'var(--foreground-strong)' },
		},
	});
</script>

<a
	{href}
	{...rest}
	aria-current={current || undefined}
	class={stylex.attrs(styles.link, current !== false && styles.current).class}
>
	<Icon {icon} size={18} stroke={1.75} />
	{label}
</a>
