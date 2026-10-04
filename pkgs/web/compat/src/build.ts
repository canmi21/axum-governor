/**
 * Reads an app's `browserslist` floors into esbuild's `build.target`.
 *
 * See spec/compat.md, "The syntax floor is set to the same line, deliberately".
 */
export function esbuildTarget(browserslist: string[]): string[] {
	return browserslist.map((query) => {
		const floor = /^([a-z]+) >= ([\d.]+)$/.exec(query);
		if (!floor)
			throw new Error(`browserslist entry is not a floor, so esbuild cannot take it: ${query}`);
		return `${floor[1]}${floor[2]}`;
	});
}
