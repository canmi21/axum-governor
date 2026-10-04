/**
 * Reads an app's `browserslist` floors into esbuild's `build.target`.
 *
 * See spec/web/compat.md, "The syntax floor is an app's `browserslist`, read into esbuild's
 * target".
 */
export function esbuildTarget(browserslist: string[]): string[] {
	return browserslist.map((query) => {
		const floor = /^([a-z]+) >= ([\d.]+)$/.exec(query);
		if (!floor)
			throw new Error(`browserslist entry is not a floor, so esbuild cannot take it: ${query}`);
		return `${floor[1]}${floor[2]}`;
	});
}
