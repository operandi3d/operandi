// Operandi User Guide footer: "Something wrong? Report it" (a new GitHub
// issue with the page name filled in) and the licence line.
(function () {
    var main = document.querySelector("#mdbook-content main") || document.querySelector("main");
    if (!main) return;
    var heading = main.querySelector("h1");
    var page = heading ? heading.textContent.trim() : document.title;
    var issue = "https://github.com/operandi3d/operandi/issues/new?title=" +
        encodeURIComponent("Guide: " + page) +
        "&body=" + encodeURIComponent("Page: " + page + "\n\nWhat's wrong:\n");
    var footer = document.createElement("footer");
    footer.className = "guide-footer";
    footer.innerHTML =
        '<p>Something wrong? <a href="' + issue + '">Report it</a>.</p>' +
        '<p>Guide text and pictures: <a href="https://creativecommons.org/licenses/by-sa/4.0/">CC BY-SA 4.0</a>. ' +
        'Code snippets and example scripts: GPL v3.</p>';
    main.appendChild(footer);
})();
