const INLINE_SCRIPT_ESCAPES: Record<string, string> = {
	'<': '\\u003C',
	'>': '\\u003E',
	'\u2028': '\\u2028',
	'\u2029': '\\u2029',
};

export function inlineScriptString(value: string): string {
	const json = JSON.stringify(value);
	if (json === undefined) throw new TypeError('a string must have a JSON representation');

	// JSON leaves characters that HTML or a JavaScript parser can treat as structure unescaped.
	return json.replace(/[<>\u2028\u2029]/g, (character) => {
		const escaped = INLINE_SCRIPT_ESCAPES[character];
		if (escaped === undefined) throw new TypeError('an inline-script escape is missing');
		return escaped;
	});
}
