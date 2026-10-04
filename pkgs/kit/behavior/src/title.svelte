<!-- A page's title: `full` for a first load in a fresh browser, `short` once the reader is known.
     See spec/kit/titles.md. -->
<script lang="ts">
	import { brevity } from './brevity.svelte.js';

	let { full, short }: { full: string; short: string } = $props();
</script>

<svelte:head>
	<title>{brevity.short ? short : full}</title>
</svelte:head>
