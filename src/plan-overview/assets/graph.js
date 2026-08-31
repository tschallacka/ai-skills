/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    function nodeDetail(node) {
        var panel = document.getElementById("node-detail");
        if (!panel) return;
        panel.hidden = false;
        panel.tabIndex = -1;
        panel.textContent = node.getAttribute("data-detail") || node.textContent;
        panel.focus();
    }
    function animateGrowth(before, after) {
        var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        var root = document.documentElement;
        root.setAttribute("data-graph-change", "growth");
        if (reduced || root.getAttribute("data-tier") === "minimal") return;
        document.querySelectorAll(".graph .node-new, .graph .edge-new").forEach(function (item) {
            item.classList.add("growth-enter");
        });
        void before; void after;
    }
    window.planGraph = { nodeDetail: nodeDetail, animateGrowth: animateGrowth };
    document.addEventListener("keydown", function (event) {
        if (event.key !== "Escape") return;
        var panel = document.getElementById("node-detail");
        if (panel) { panel.hidden = true; }
    });
}());
