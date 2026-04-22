(function () {
  var saved = localStorage.getItem("theme");
  if (saved) document.documentElement.dataset.theme = saved;

  function toggleTheme() {
    var html = document.documentElement;
    html.dataset.theme = html.dataset.theme === "dark" ? "light" : "dark";
    localStorage.setItem("theme", html.dataset.theme);
    document.dispatchEvent(new CustomEvent("themechange"));
  }

  document.addEventListener("DOMContentLoaded", function () {
    var btn = document.getElementById("theme-toggle");
    if (btn) btn.addEventListener("click", toggleTheme);
  });
})();