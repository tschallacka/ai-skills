/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    var lastState = null;
    function expanded() { return Array.prototype.map.call(document.querySelectorAll("[aria-expanded='true'][id]"), function (n) { return n.id; }); }
    function applyStateChange(next) {
        var saved = { hash: window.location.hash, scrollY: window.scrollY, expanded: expanded(), tab: window.planAutoplay && window.planAutoplay.selected };
        if (lastState && window.planGraph) window.planGraph.animateGrowth(lastState, next);
        lastState = next;
        var root = document.documentElement;
        root.setAttribute("data-generated-at", next.generatedAt || "");
        var marker = document.getElementById("last-updated");
        if (marker) marker.textContent = "Updated " + (next.generatedAt || "just now");
        saved.expanded.forEach(function (id) { var node = document.getElementById(id); if (node) node.setAttribute("aria-expanded", "true"); });
        window.scrollTo(0, saved.scrollY);
        if (window.location.hash !== saved.hash) window.location.hash = saved.hash;
        if (window.planAmbient) window.planAmbient.activityPulse();
    }
    window.planLive = { applyStateChange: applyStateChange };
}());
