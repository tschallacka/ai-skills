/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    var samples = [], windowSize = 30;
    function frameSampler(frameMs) {
        samples.push(frameMs);
        if (samples.length > windowSize) samples.shift();
        return samples.length ? samples.reduce(function (sum, value) { return sum + value; }, 0) / samples.length : 0;
    }
    function applyTier(tier, cause) {
        var root = document.documentElement;
        var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        var selected = reduced ? "minimal" : tier;
        root.setAttribute("data-tier", selected);
        var indicator = document.getElementById("cinematic-tier");
        if (indicator) indicator.textContent = "Cinematic tier: " + selected + " (" + (reduced ? "reduced motion" : cause || "measured") + ")";
    }
    window.planPerformance = { frameSampler: frameSampler, applyTier: applyTier };
}());
