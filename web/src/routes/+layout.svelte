<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<header class="top">
  <a class="brand" href="/">aion<span class="mark">·</span>context<span class="sub">studio</span></a>
  <nav class="nav">
    <a href="/">Workspace</a>
    <a href="/registry">Registry</a>
  </nav>
</header>

<main class="app">
  {@render children()}
</main>

<footer class="foot">
  Local single-operator demo — keys are held server-side for the demo only and never leave loopback.
  Not a production custody architecture. Claude advises; humans sign.
</footer>
