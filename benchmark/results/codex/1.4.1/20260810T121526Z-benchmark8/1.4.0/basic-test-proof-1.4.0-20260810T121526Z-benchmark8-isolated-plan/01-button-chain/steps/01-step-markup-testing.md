# Testing companion: 01-step-markup

## Browser verification

This target is verified as part of US-01. In a fresh browser context, open the planned local `button-chain.html` route and confirm that exactly one visible button is present before any click. Pass only when the initial control is visible and is the control named by the user story cache; fail if the page starts with zero or multiple buttons.

## Automated tests

No separate automated test is planned for this standalone markup target. The required proof is the direct browser assertion in US-01, owned by W04.
