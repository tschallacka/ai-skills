/* MODE: DEV */
/* PACKAGE: PROD */
(function () {
    "use strict";
    function activityPulse() {
        var marker = document.getElementById("activity-pulse");
        if (!marker) return;
        marker.textContent = "Live update received";
        marker.setAttribute("data-active", "true");
        marker.classList.remove("pulse");
        void marker.offsetWidth;
        marker.classList.add("pulse");
    }
    window.planAmbient = { activityPulse: activityPulse };
}());
