# Testing: 01-step-browser-proof

## Browser verification

- Future execution only: start from one initial button and use five real mouse clicks or keyboard activations on the visible current last button.
- After clicks 1–4, require total visible button counts 2, 3, 4, and 5, with the new button below its predecessor.
- After click 5, require zero buttons, no prior document content, exact lowercase `finished`, and a visibly white border.
- Save the route and one decisive final screenshot. Console evaluation, script injection, DOM mutation, direct APIs, and injected events are prohibited.
- Do not start a browser, server, or driver during the planning-only proof.
