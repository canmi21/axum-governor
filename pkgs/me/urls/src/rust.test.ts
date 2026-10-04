import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { rustCanmi } from './rust.ts';

/** Both sides with each constant on one line, however rustfmt chose to lay it out. */
function constants(rust: string): string {
	return rust.replace(/=\s+"/g, '= "');
}

it('keeps the committed Rust half in step with the map', () => {
	const committed = readFileSync(
		new URL('../../../../crates/canmi/src/lib.rs', import.meta.url),
		'utf8',
	);
	// A mismatch means the map changed without `mise run urls` -- regenerate rather than edit.
	expect(constants(committed)).toBe(constants(rustCanmi()));
});
