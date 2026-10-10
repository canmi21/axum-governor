/**
 * The contract's names, by group: every style gives each a value in light and in dark, and a
 * component reads these and no color of its own. See lib's spec/design/styles.md.
 */
export const CONTRACT = [
	{
		group: 'background',
		names: [
			'--background-base',
			'--background-inset',
			'--background-surface',
			'--background-raised',
		],
	},
	{ group: 'border', names: ['--border-subtle', '--border-default', '--border-strong'] },
	{
		group: 'foreground',
		names: [
			'--foreground-subtle',
			'--foreground-muted',
			'--foreground-default',
			'--foreground-strong',
		],
	},
	{
		group: 'interaction',
		names: [
			'--interaction-hover',
			'--interaction-selected',
			'--interaction-selected-opaque',
			'--interaction-focus',
		],
	},
	{ group: 'accent', names: ['--accent'] },
	{
		group: 'status',
		names: ['--status-success', '--status-info', '--status-warning', '--status-critical'],
	},
	{ group: 'shadow', names: ['--shadow-surface'] },
	{ group: 'font', names: ['--font-sans', '--font-mono', '--font-sans-cap-height'] },
] as const;

export type Group = (typeof CONTRACT)[number]['group'];
export type Name = (typeof CONTRACT)[number]['names'][number];
