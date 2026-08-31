/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    var backStack = [];
    var current = window.location.hash.slice(1) || "overview";
    var invokingLink = null;
    function expandedState() {
        return Array.prototype.map.call(document.querySelectorAll("[aria-expanded='true'][id]"), function (node) { return node.id; });
    }
    function save() { return { hash: current, scrollY: window.scrollY, expanded: expandedState() }; }
    function restore(snapshot) {
        snapshot.expanded.forEach(function (id) { var node = document.getElementById(id); if (node) node.setAttribute("aria-expanded", "true"); });
        window.scrollTo(0, snapshot.scrollY);
    }
    function navigate(hash) { backStack.push(save()); current = hash; window.location.hash = hash; }
    function back() { var previous = backStack.pop(); if (previous) { current = previous.hash; window.location.hash = current; restore(previous); } else window.location.hash = "overview"; }
    function peek(link) {
        invokingLink = link;
        var panel = document.getElementById("peek");
        if (!panel) return;
        panel.textContent = link.getAttribute("data-peek") || "Related item";
        panel.hidden = false;
        panel.focus();
    }
    window.planNavigation = { backStack: backStack, navigate: navigate, back: back, peek: peek };
    window.addEventListener("keydown", function (event) {
        if (event.key !== "Escape") return;
        var panel = document.getElementById("peek");
        if (panel && !panel.hidden) { panel.hidden = true; if (invokingLink) invokingLink.focus(); }
    });
    window.addEventListener("hashchange", function () { current = window.location.hash.slice(1) || "overview"; });
}());
