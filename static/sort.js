(function () {
  var ASC = "asc";
  var DESC = "desc";

  function cellValue(row, colIndex) {
    var cell = row.cells[colIndex];
    return cell ? cell.getAttribute("data-sort") || cell.textContent.trim() : "";
  }

  function compareValues(a, b) {
    var na = parseFloat(a.replace(/[^0-9.\-]/g, ""));
    var nb = parseFloat(b.replace(/[^0-9.\-]/g, ""));
    if (!isNaN(na) && !isNaN(nb)) return na - nb;
    return a.localeCompare(b);
  }

  function sortTable(table, colIndex, dir) {
    var tbody = table.tBodies[0];
    if (!tbody) return;
    var rows = Array.prototype.slice.call(tbody.rows);
    rows.sort(function (a, b) {
      var va = cellValue(a, colIndex);
      var vb = cellValue(b, colIndex);
      var cmp = compareValues(va, vb);
      return dir === ASC ? cmp : -cmp;
    });
    rows.forEach(function (row) {
      tbody.appendChild(row);
    });
  }

  function initTable(table) {
    if (table._sortInit) return;
    table._sortInit = true;

    var headers = table.tHead && table.tHead.rows[0]
      ? Array.prototype.slice.call(table.tHead.rows[0].cells)
      : [];

    headers.forEach(function (th, colIndex) {
      th.setAttribute("data-sort-dir", "");
      th.style.cursor = "pointer";
      th.style.userSelect = "none";
      th.setAttribute("title", "Click to sort");

      th.addEventListener("click", function () {
        var currentDir = th.getAttribute("data-sort-dir");
        var newDir = currentDir === ASC ? DESC : ASC;

        // Reset all headers
        headers.forEach(function (h) {
          h.setAttribute("data-sort-dir", "");
          h.querySelector(".sort-arrow") && (h.querySelector(".sort-arrow").textContent = "");
        });

        th.setAttribute("data-sort-dir", newDir);

        var arrow = th.querySelector(".sort-arrow");
        if (!arrow) {
          arrow = document.createElement("span");
          arrow.className = "sort-arrow";
          th.appendChild(arrow);
        }
        arrow.textContent = newDir === ASC ? " ▲" : " ▼";

        sortTable(table, colIndex, newDir);
      });

      // Add arrow placeholder span
      var arrow = document.createElement("span");
      arrow.className = "sort-arrow";
      th.appendChild(arrow);
    });
  }

  function initAll() {
    document.querySelectorAll("table.striped").forEach(initTable);
  }

  document.addEventListener("DOMContentLoaded", initAll);
  document.addEventListener("htmx:afterSettle", initAll);
})();