# Verification: 05-step-browser-verification

## Automated tests

Run the planned browser flow only during the future implementation phase: open button-chain.html and perform five real user clicks in this order: initial button, generated button one, generated button two, generated button three, then generated button four. The first four clicks must each append exactly one generated button below the previous last button. The fifth click, on the fourth generated button after it exists, must clear the document and show exact lowercase finished with a visible white border. Record pass/fail evidence for each click count, the final clearing behavior, exact text, and border visibility.
