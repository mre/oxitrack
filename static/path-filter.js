// Client-side path filter for the Pages table. The input lives inside the
// htmx-swapped `#stats-panel`, so it is re-rendered on every range change.
// Rather than re-binding after each swap, we listen once via event delegation
// and resolve the target table relative to the input that fired the event.
(function () {
  function applyFilter(input) {
    var section = input.closest("section");
    if (!section) return;
    var table = section.querySelector("table.striped");
    if (!table || !table.tBodies[0]) return;

    var query = input.value.trim().toLowerCase();
    Array.prototype.forEach.call(table.tBodies[0].rows, function (row) {
      var cell = row.cells[0];
      var text = cell ? cell.textContent.toLowerCase() : "";
      row.hidden = query !== "" && text.indexOf(query) === -1;
    });
  }

  document.addEventListener("input", function (e) {
    if (e.target && e.target.classList.contains("path-filter__field")) {
      applyFilter(e.target);
    }
  });
})();
