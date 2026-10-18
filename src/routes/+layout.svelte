<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<div class="bg-surface-900 text-slate-100 font-sans antialiased h-screen w-screen overflow-hidden select-none flex flex-col">
  {@render children()}
</div>
