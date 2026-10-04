import * as stylex from '@stylexjs/stylex';
import {
	border,
	duration,
	easing,
	leading,
	radius,
	text,
	transition,
	weight,
} from './vocabulary.stylex.ts';

/**
 * The declaration groups the visual layer repeats, each with one name -- `vocabulary.stylex.ts`
 * names the values, this names the sets built from them, at the same three-component threshold.
 * A group holds whole properties only, never a conditional branch over a default that differs
 * per site -- see spec/architecture/css/extraction.md, "The merge unit is the property, not the
 * property and its condition". A key's spelling is not part of the stylesheet, so a drifted name is
 * cheap to fix and easy to leave wrong.
 */

/**
 * The four declarations of a bordered paper surface, which eight components draw. Kept out of
 * `stylex.create` so `blockFrame` below is visibly the same thing with a corner. The radius is
 * not among them: no two of the eight agree on one. `borderStyle` is stated rather than omitted
 * -- see spec/architecture/css/extraction.md, "An omitted longhand is not its initial value when a
 * shorthand registered it". `hairlinePx` is what excludes the cargo and tokei tooltips, which draw
 * the same ground with `hairlineRem`. See spec/todo/todo.md.
 */
const paper = {
	backgroundColor: 'var(--color-paper)',
	borderWidth: border.hairlinePx,
	borderStyle: 'solid',
	borderColor: 'var(--color-border)',
};

/**
 * The child half of a focus ring a control hands down: the host sets the `--focus-ring-*` group
 * at `:focus-visible` and inheritance carries it here, so both ends are same-element declarations
 * and no selector crosses between them. The colour is stated rather than handed down -- an
 * outline's is `currentColor` until named, and `transition.colors` carries `outline-color`. See
 * spec/styling/focus.md.
 */
const handedRing = {
	outlineStyle: 'var(--focus-ring-style, none)',
	outlineWidth: 'var(--focus-ring-width, medium)',
	outlineColor: 'var(--color-accent)',
};

export const surfaces = stylex.create({
	/**
	 * The ground every route stands on: the page's own colour and the ink that inherits from it.
	 * This removes the duplication of seven components writing the same two declarations, but not
	 * the question underneath it -- one line on `body` would be the better repair. See
	 * spec/todo/css.md, "The page ground is now one name, and the repair it is standing in for is one
	 * line on `body`".
	 */
	page: {
		backgroundColor: 'var(--color-page)',
		color: 'var(--color-text)',
	},

	paper,

	/**
	 * The frame a full-width block draws around itself: the code block, the Mermaid figure and
	 * the quadrant. `paper` with a corner, written as the spread so the two cannot part company --
	 * all three carried one Tailwind string before the migration, hidden overflow and the `xl`
	 * corner over a `border-border` hairline on the paper ground, so this is one decision
	 * translated three times. The search panel's identical five are deliberately not this: a panel
	 * over the page is not a block in an article, and the corner is the only thing they share.
	 */
	blockFrame: { ...paper, borderRadius: radius.xl },

	/**
	 * The panel a menu opens on: `paper` with a medium corner, square rows running edge to edge
	 * inside it. The site's dropdowns and the CMS's context menu are one kind of thing and draw
	 * this, which is a name opened by that decision rather than found by the three-component bar --
	 * see spec/architecture/css/extraction.md, "A name can be decided as well as found". The shadow
	 * is the markup's `shadow-sm`, for the reason `menu-content.svelte` gives.
	 */
	menu: { ...paper, borderRadius: radius.md },

	/**
	 * Bordered paper that answers a pointer: the repository card, the tweet card and the support pill
	 * each darken the border and the ground on hover and focus. The `hairlineRem` is not a ruling
	 * against `paper`'s `hairlinePx` -- spec/todo/css.md, "Two named surfaces disagree about what a
	 * hairline is, and each is internally consistent". `:hover` is bare, with no `(hover: hover)`,
	 * matching the rules it replaced (sameness first; spec/architecture/css/migration.md). The
	 * transition and the radius are not here: neither is common to all three.
	 */
	interactive: {
		borderWidth: border.hairlineRem,
		borderStyle: 'solid',
		borderColor: {
			default: 'var(--color-border)',
			':hover': 'var(--color-border-strong)',
			':focus-visible': 'var(--color-border-strong)',
		},
		backgroundColor: {
			default: 'var(--color-paper)',
			':hover': 'var(--color-paper-hover)',
			':focus-visible': 'var(--color-paper-hover)',
		},
	},

	/**
	 * How a colour change is drawn: eight components hand their hover and focus colours this
	 * curve, and before this they each wrote the three declarations out. Named for the cost of
	 * changing it rather than for bytes -- atomic classes already deduplicate these, so nothing
	 * shrinks, but the curve was eight files to edit and is now one. `quietControl` below keeps
	 * its own, which names two properties rather than `transition.colors`' ten.
	 */
	colorShift: {
		transitionProperty: transition.colors,
		transitionDuration: duration.base,
		transitionTimingFunction: easing.inOut,
	},

	/**
	 * The compact icon-and-label control a metadata row is made of. See spec/styling/focus.md, "Quiet
	 * metadata controls share one surface" (why this replaced the `.quiet-control` class) and
	 * spec/todo/todo.md, "A recipe's other half is a convention and nothing checks that a call site
	 * kept it" (why only the appearance is here). `:hover` is bare (sameness first,
	 * spec/architecture/css/migration.md). `outline: none` also resets width and colour, to `medium`/
	 * `currentcolor` rather than the `0.125rem` accent base-layer `:focus-visible` sets.
	 */
	quietControl: {
		// A member of this surface, which seven files apply: the cursor arrives with the colour,
		// the radius and the transitions, or none of them does. See spec/architecture/css/layers.md,
		// "A name alone is not a recipe".
		cursor: 'pointer',
		borderRadius: '0.125rem',
		color: {
			default: 'var(--color-text-soft)',
			':hover': 'var(--color-text-strong)',
			':focus-visible': 'var(--color-text-strong)',
		},
		backgroundColor: {
			default: null,
			':hover': 'var(--color-paper-hover)',
			':focus-visible': 'var(--color-paper-hover)',
		},
		// Doubled because two properties transition: spec/todo/todo.md, "A `transition` shorthand
		// sets five lists and the migrated form writes three".
		transitionProperty: 'color, background-color',
		transitionDuration: '200ms, 200ms',
		transitionTimingFunction: 'ease, ease',
		transitionDelay: '0s, 0s',
		outlineStyle: { default: null, ':focus-visible': 'none' },
		outlineWidth: { default: null, ':focus-visible': 'medium' },
		outlineColor: { default: null, ':focus-visible': 'currentColor' },
	},

	/**
	 * A control whose visible shape is a child rather than its own box: it declares the ring the
	 * child draws instead of drawing one, and takes the base layer's backstop off itself so it
	 * does not wear two. Which of the three the child reads is the child's own affair. The
	 * `outline` longhands are `outline: none` written out, for the reason `quietControl` gives.
	 * See spec/styling/focus.md.
	 */
	focusRingHost: {
		// Read through `--focus-ring-suppress` rather than stated outright. A known pointer takes
		// the ring away on the focused element, and the ring being suppressed here is drawn on a
		// child of it, which no selector in that rule reaches -- so it writes that name instead,
		// which nothing else writes and no cascade decides. `none` is not a width, so the child's
		// `outline-width` is invalid at computed-value time and falls back to its initial `medium`
		// -- the reset `outline: none` performs. See spec/styling/focus.md.
		'--focus-ring-style': { default: null, ':focus-visible': 'var(--focus-ring-suppress, solid)' },
		'--focus-ring-width': {
			default: null,
			':focus-visible': 'var(--focus-ring-suppress, 0.125rem)',
		},
		// Not suppressed: a corner is not a ring, and the pointer rule never took it away.
		'--focus-ring-radius': { default: null, ':focus-visible': radius.sm },
		outlineStyle: { default: null, ':focus-visible': 'none' },
		outlineWidth: { default: null, ':focus-visible': 'medium' },
		outlineColor: { default: null, ':focus-visible': 'currentColor' },
	},

	/** The child a host's ring is drawn on, where the child's corner is already its own. */
	focusRingInner: handedRing,

	/**
	 * The same for a text link's inner span, whose corner comes from the host too: a corner left
	 * on at rest would clip the underline's background, and it is sized for the outline rather
	 * than for the stroke. See spec/styling/focus.md.
	 */
	focusLinkInner: { ...handedRing, borderRadius: 'var(--focus-ring-radius, 0)' },

	/**
	 * The interface's own type step: fourteen pixels with the line that step computes to. Six
	 * components take the pair; the other sites of `text.px14` override the line on purpose, so
	 * it is the pair that marks interface text. The line is a length rather than the ratio
	 * Tailwind's fourteen-pixel step writes, which does not terminate and rounds short -- see
	 * spec/architecture/css/authoring.md, "A ratio that does not terminate cannot be written as a
	 * ratio". `leading.px20` has no site that is not one of these six.
	 */
	uiText: {
		fontSize: text.px14,
		lineHeight: leading.px20,
	},

	/**
	 * What a heading is made of: two declarations and no size, admitted on identity rather than
	 * on length -- twelve keys write exactly this pair, ten of them as the whole style object.
	 * Not the support pill, which writes the same pair but is a control: naming it `heading` would
	 * be the drift the file-opening comment warns a wrong name invites.
	 */
	heading: {
		color: 'var(--color-text-strong)',
		fontWeight: weight.medium,
	},
});
