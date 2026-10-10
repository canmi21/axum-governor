/**
 * Whether a page's title may be its short one. See spec/kit/titles.md.
 */
import type { Container, Store } from './state.ts';

/** In a record paired with `localStorage`: this browser has loaded a page here before. */
const SEEN = 'visit.seen';

class Brevity {
	short = $state(false);
}

/** False on the server and through hydration, so both render the full title. */
export const brevity = new Brevity();

/** Once the first page has hydrated: short if this browser was here before; marked either way. */
export function settleBrevity(record: Container, storage: Store = localStorage): void {
	brevity.short = record.recall(storage, SEEN, false);
	record.remember(storage, SEEN, true);
}

/** After a navigation inside the app, which a crawler never makes. */
export function shortenTitles(): void {
	brevity.short = true;
}
