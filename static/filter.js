// All filter state is owned by the server: clicking a preset button issues an
// hx-get whose response carries the new HTML and an `HX-Push-Url` header that
// updates the address bar. Active-button styling is rendered server-side too,
// so this file does not participate in any of that.
//
// The only remaining concern is the live-presence row dots, which are updated
// from an out-of-band swap performed by `/api/live`. The `#live-path-set`
// element is replaced wholesale; we just translate its `data-paths` payload
// into per-row CSS state.
(function () {
  document.addEventListener("htmx:oobAfterSwap", function (e) {
    if (!e.detail.target || e.detail.target.id !== "live-path-set") return;
    var activePaths = new Set(
      JSON.parse(e.detail.target.dataset.paths || "[]"),
    );
    document.querySelectorAll(".live-row-dot").forEach(function (dot) {
      dot.classList.toggle(
        "live-row-dot--active",
        activePaths.has(dot.dataset.path),
      );
    });
  });
})();
