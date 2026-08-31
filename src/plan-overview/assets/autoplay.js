/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    var autoplay = false;
    var selected = null;
    var tabs = {};
    function autoplayTabs(states) {
        var next = {};
        states.forEach(function (state) { next[state[0]] = state; });
        tabs = next;
        if (!selected || !tabs[selected]) selected = states.length ? states[0][0] : null;
        var strip = document.getElementById("autoplay-tabs");
        if (!strip) return;
        strip.innerHTML = "";
        Object.keys(tabs).forEach(function (id) {
            var button = document.createElement("button");
            button.textContent = id;
            button.setAttribute("aria-pressed", id === selected ? "true" : "false");
            button.onclick = function () { selected = id; autoplayTabs(states); autoplayFollow(states); };
            strip.appendChild(button);
        });
        if (!states.length) strip.textContent = "No active step.";
    }
    function autoplayFollow(states) {
        autoplayTabs(states);
        if (!autoplay || !selected) return;
        window.location.hash = "unit/" + encodeURIComponent(selected);
        var marker = document.getElementById("autoplay-status");
        if (marker) marker.textContent = "Following " + selected;
    }
    window.planAutoplay = {
        autoplayFollow: autoplayFollow,
        autoplayTabs: autoplayTabs,
        setEnabled: function (enabled) { autoplay = enabled; if (enabled) autoplayFollow(Object.keys(tabs).map(function (id) { return tabs[id]; })); }
    };
}());
