// Swap favicon for dark/light mode
// Handles both OS preference and pydata-sphinx-theme toggle
(function() {
  function update() {
    var link = document.querySelector("link[rel='icon']");
    if (!link) return;
    var theme = document.documentElement.dataset.theme;
    if (!theme) {
      theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    var base = link.href.replace(/favicon(-dark)?\.png/, 'favicon.png');
    link.href = theme === 'dark' ? base.replace('favicon.png', 'favicon-dark.png') : base;
  }

  // Run on load
  update();

  // Watch OS preference
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', update);

  // Watch pydata-sphinx-theme toggle (mutates data-theme attribute)
  new MutationObserver(update).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme']
  });
})();
