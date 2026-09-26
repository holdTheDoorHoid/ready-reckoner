// Applies a saved light/dark choice before the page paints, so a manual override never flashes
// the other theme. A separate file (not inline) because the Content Security Policy forbids
// inline scripts. Reads only the display preference, never the plan.
try {
  var prefs = JSON.parse(localStorage.getItem('rr.prefs.v1') || '{}');
  if (prefs.theme === 'light' || prefs.theme === 'dark') {
    document.documentElement.setAttribute('data-theme', prefs.theme);
  }
} catch (e) {
  // Storage blocked (private mode): follow the system setting.
}
