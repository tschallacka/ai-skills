# Verification: 01-step-create-markup

## Static verification

Command for the future executor:

```bash
test -f button-chain.html
rg -n 'id="button-chain-app"|id='\''button-chain-app'\''' button-chain.html
rg -n '<button\\b' button-chain.html
```

Pass only if `button-chain.html` exists, the load-state markup has the `#button-chain-app` scope, and the static initial DOM defines exactly one initial visible button with no pre-rendered generated buttons and no visible `finished` message. This static check does not replace W05 browser verification.
