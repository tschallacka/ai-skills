---
name: sanitairkamer-deviations
description: Project-specific ways this Magento 2 store (sanitairkamer) deviates from a stock Magento install - theme layout, custom checkout wiring, EAV attribute gaps, Amasty ExtraFee quirks, and test-environment specifics. Load before writing tests or debugging anything checkout/EAV/CSS-related on this project, to avoid re-discovering the same things every session.
---
<!-- MODE: DEV -->
<!-- PACKAGE: DEV -->

# Sanitairkamer project-specific deviations from default Magento

Facts learned the hard way, kept here so they don't need re-discovering. If a
fact stops being true, fix it here rather than leaving it stale.

## Checkout theme

- Checkout runs under its own separate, **non-Hyva** theme
  (`Sanitairkamer/default`), a Luma/Knockout compatibility layer
  (`hyva-themes/magento2-luma-checkout`) - distinct from the main storefront's
  Hyva theme. Product/category pages are Hyva (Alpine.js, no jQuery);
  checkout is genuine jQuery/Knockout.
- The checkout address form is a custom postcode-lookup widget, not stock
  Luma: a single "Voer handmatig je adres in" checkbox (target its
  `label[for='manualAddress']`, not the checkbox input directly - manual
  street/city fields are hidden until checked) reveals split fields
  `street[0]` (Straatnaam) and `street[1]` (Huisnr) instead of one combined
  street line.
- The step-1 -> step-2 ("Verzendmethode") continue button is the **standard**
  Magento one: `button[data-role='opc-continue']`.
- The step-2 -> step-3 ("Betaling") continue button is **not** the standard
  one. This theme replaces it with a custom BigBridge toolbar region
  (`bb-toolbar`); the actually-visible, clickable button is
  `button[data-bind='click: continueToPayment']`. The standard
  `button[data-role='opc-continue']` element still exists in the DOM at that
  point (hidden/unused) - using it there throws
  `ElementNotInteractableException`, and worse, since BOTH buttons share the
  literal attribute selector `[data-role='opc-continue']` across different
  steps, generic clicks on that selector are ambiguous once more than one
  step's markup is present. Always confirm which continue button is actually
  visible for the step you're on rather than assuming the selector that
  worked for step 1 also works for step 2.
- `BigBridge_ShippingLogic`'s own checkout widget (`shipping-logic.js` +
  `delivery-moments/default.js`) renders the delivery-date/timeslot picker on
  the **shipping-method step**, not the address step. Filling the address
  alone does not reveal it - an explicit click on the step-1 continue button
  is required first.
- Custom radio options in this widget (own-transport day/morning/afternoon,
  evening delivery, etc.) use the common "visually-hidden real input, styled
  label is the actual control" pattern: the `<input>` is 1x1px and clipped
  (`position:absolute; clip: rect(0,0,0,0)`). Target the wrapping
  `label.delivery-moment__time-slot[for='<id>']` for both waiting and
  clicking - waiting on/clicking the raw `#id` input selector will time out
  or fail as not-interactable even though the feature works fine for a real
  user.
- Selecting a delivery timeslot triggers an async totals recalculation
  (Amasty ExtraFee condition re-evaluation) that can briefly make the next
  step's continue button not interactable if clicked immediately after -
  add a small settle wait (a `waitForLoadingMaskToDisappear`, though note
  this widget's own recalculation doesn't always use the generic
  `.loading-mask` the helper watches for - verify it's actually catching the
  right indicator) or an explicit short wait before continuing.

## CSS / theme file resolution

- `app/design/frontend/Sanitairkamer/default/BigBridge_ShippingLogic/web/css/styles.css`
  is a **theme-level override** that shadows the module's own
  `app/code/BigBridge/ShippingLogic/view/frontend/web/css/styles.css` for the
  layout XML reference `<css src="BigBridge_ShippingLogic::css/styles.css"/>`.
  The two files have already drifted apart (the theme copy has extra rules
  the module copy lacks). Before editing this module's CSS, check
  `checkout_index_index.xml`'s `<css src="..."/>` to confirm which physical
  file actually applies - editing only the module copy silently does nothing
  for checkout.
- After any CSS/LESS change on this project, `bin/magento cache:flush` is
  **not sufficient** to see it reflected - static content can keep serving a
  stale compiled copy, or the page can render fully unstyled right after a
  flush. Run the workbench's full rebuild instead:
  `/opt/workbench/profile/bin/magento-cc` (aliased `cc` in interactive
  shells only - use the full path in non-interactive tool calls). It wipes
  `pub/static`, `var/view_preprocessed`, `var/cache`, `var/generation`,
  `generated/code`, recompiles every theme's LESS/Tailwind, then flushes
  Magento's cache. Still hard-reload the browser afterward (e.g.
  ctrl+shift+r) - a stale cached copy of the previous static-version's file
  can linger client-side even after the server-side rebuild.
- The theme's generic totals-table styling
  (`.data.table.table-totals td { text-align: right }`) targets *any* `<td>`
  in that table, since normally only amount cells are `<td>` (label cells are
  `<th>`). A custom row using a `<td>` for its label (e.g. Amasty ExtraFee's
  breakdown row) inherits this right-alignment and needs an explicit
  `text-align: left !important` override - a same-or-lower-specificity
  plain-class override loses the cascade battle.

## Custom/legacy EAV attributes

- Several custom product attributes used throughout `BigBridge_ShippingLogic`
  business logic - `verzendtype` (shipping type: "Eigen vervoer"/own-transport
  vs "Pakketdienst"/courier), `leverancier` (supplier), `deliverytime`,
  `levertijd_2` (out-of-stock delivery time), and `xcore_suppliers_purchase_price`
  (from `BigBridge_XcoreAttributeCopy`) - **exist in production but are not
  created by any setup script or data patch anywhere in this codebase**. They
  predate the current codebase and were created out-of-band. A freshly
  installed database (integration test DB, a new environment) will not have
  them at all.
  - For integration tests: create minimal stand-in attributes via
    `EavSetupFactory`/`EavSetup::addAttribute()` in a shared fixture trait
    (see `OwnTransportProductFixtureTrait`). Critical gotcha: `addAttribute()`
    with `'user_defined' => true` and no explicit `'group'` key silently
    skips assigning the attribute to any attribute set, so EntityManager
    never persists a value for it - always pass `'group' => 'General'` (or
    similar) alongside `user_defined`.
  - For MFTF tests: prefer a real, already-imported catalog product SKU over
    `createData` fixtures, since faking ~10 required custom attributes isn't
    worth it for what these tests actually verify.
  - `BigBridge\XcoreAttributeCopy`'s post-save observer crashes
    ("Call to a member function getBackend() on bool") if
    `xcore_suppliers_purchase_price` doesn't exist - set
    `xcore_stop_cost_price_sync` = 1 on fixture products to skip it.

## Amasty ExtraFee

- Ships **no Dutch translation at all** (only `i18n/en_US.csv`) - strings like
  "Additional Fees"/"Additional fees" render in English on this Dutch store
  unless translated via this project's own module i18n CSV.
- Its data model only supports customer-facing **selectable** fee types
  (`FRONTEND_TYPE_CHECKBOX`/`_DROPDOWN`/`_RADIO`) - there is no "silent,
  automatic, no customer choice" fee type. For a fee that should apply
  automatically based on a backend condition (no customer toggle), the whole
  "Additional Fees" selectable block UI (`[data-amexfee-js="block"]`) needs
  hiding via CSS; the plain totals-summary row
  (`checkout.sidebar.summary.totals.fee`, template
  `checkout/summary/totals/fee.html`) still shows the charged amount
  correctly on its own.
- That totals-summary row itself renders **two** rows when a fee applies: an
  aggregate group-total row (`tr.amexfee-collapsible-block`) plus a
  collapsed-by-default itemized breakdown row (`tr.amexfee-totals-details`,
  populated via `ko foreach: items` - genuinely absent from the DOM, not just
  hidden, when no fee item matches). With only one fee ever active at a time,
  showing both is a duplicated price - hide the aggregate row and force the
  breakdown row to always display instead of staying collapsed.
- Rule conditions are registered per-project via the
  `salesrule_rule_condition_combine` event (see
  `BigBridge_ShippingLogic`'s `Observer\Admin\AddOwnTransportTimeslotCondition`
  / `etc/adminhtml/events.xml`) - Amasty's `Combine` condition tree is
  `Magento\SalesRule\Model\Rule\Condition\Combine`, extensible the same way
  core sales rules are.
- Amasty `Fee` entities are created via `FeeFactory`/`FeeRepository` in a data
  patch (see `Setup/Patch/Data/AddOwnTransportTimeslotFee.php`). If a
  condition needs to distinguish between multiple specific values (e.g.
  morning vs afternoon), make the custom condition's `validate()` expose the
  raw underlying value (not a collapsed boolean) and create **one Fee entity
  per distinct value** with its own condition/label/option, rather than one
  fee covering multiple values with an ambiguous combined label.

## Other domain-specific gotchas

- `BigBridge\MageUtils\Collection\CollectionBase` (a Laravel-Collection-style
  helper) `->map()` returns another `CollectionBase`, not a plain array -
  passing it to native `array_map()` throws a `TypeError`.
- `BigBridge\ShippingLogic\Plugin\QuoteShippingBucketPlugin::afterGet()`
  populates the `shipping_logic_buckets` extension attribute as a genuine
  plain array **only** when the quote is freshly loaded through
  `CartRepositoryInterface::get()`. Reusing an in-memory quote object that a
  bucket-filling observer/service just processed retains a `CollectionBase`
  instead - reload via the repository before relying on that extension
  attribute's shape (e.g. before passing it through code that expects a
  plain array).
- A missing/unbalanced `<!-- ko if -->` / `<!-- /ko -->` pair in a shared
  Knockout template breaks rendering for **the entire template** silently -
  no console error, no exception, just blank/unbound content for every
  instance of that component regardless of which conditional branch should
  have applied. When editing a shared `.html` KO template, sanity-check that
  `grep -c "<!-- ko "` and `grep -c "<!-- /ko"` match before assuming a
  rendering bug is a JS/data problem.
- PHPUnit's static-property backup/restore between tests breaks with
  **typed** static properties (`private static bool $x`) -
  `TypeError: Cannot assign null to property ... of type bool`. Avoid
  typed-static caching flags in test traits/classes; if idempotency-checking
  is cheap enough, just re-check it every call instead of caching a "done"
  flag.
- `createMock()`'s `willReturnMap()` matches on a method's full declared
  parameter list, including unpassed optional parameters - not just the
  arguments actually supplied at the call site. `Order::getData($key = '',
  $index = null)` called as `getData('foo')` still needs a map row of
  `['foo', null, $returnValue]` (3 elements), not `['foo', $returnValue]`
  (2) - the 2-element form silently never matches and the mocked method
  returns `null`.

## Customer-facing delivery-time/timeslot text (order emails)

- The order confirmation/shipment emails' "Verwachte levering: ..." line is
  rendered by a separate module, `Cream\DeliveryTime` (not
  `BigBridge\ShippingLogic`), via `Helper\Product::getDeliveryPromiseForOrder()`,
  called from `Cream_DeliveryTime::delivery_time.phtml`, embedded in
  `order_new(_guest)`/`shipment_new(_guest)` email templates for both the
  Luma (`Sanitairkamer/default`) and Hyva-mail (`theme-{sanitairkamer,
  gichaa}-hyva`) themes. It reads the flat `expected_deliverydate`/
  `expected_delivery_timeslot` order columns directly (not the
  `shipping_logic_buckets` extension attribute or the bucket's own
  `delivery_promise` string) and maps `ShippingWindowType` values to Dutch
  compound-word suffixes/labels (day+ochtend/middag/avond, Van-/Morgen-
  variants for today/tomorrow) via lookup tables keyed by the int enum value.
- The Amasty ExtraFee surcharge line shows up in these same emails
  automatically via the stock totals renderer wired into the
  `sales_email_order_items` layout handle - no project code needed for that
  part.

## REST API data flow

- `Plugin\Magento\Checkout\Model\GuestShippingInformationManagement` runs
  *before* core's own masked-id resolution, so its `$cartId` is still the
  masked string, not the real quote id - resolve it yourself
  (`MaskedQuoteIdToQuoteId::execute()`) before loading the quote. The
  non-guest `ShippingInformationManagement` plugin's `$cartId` is already
  the real numeric id.
- `Observer\BucketFiller` only fires on storefront predispatch events
  (`etc/events.xml`) - never for pure REST/API calls. Anything needing
  `shipping_logic_buckets` on a quote reached only via REST must call
  `BucketFiller::recalculateShippingBuckets($quote)` directly when buckets
  are empty; that method is safe standalone (only `execute()`, the event
  observer, touches `CheckoutSession`).
- `Api\Data\ShippingBucketInterface::getShippingWindow()`/
  `setShippingWindow()` must keep an `int` docblock type, not `bool` -
  Magento's webapi `TypeProcessor` serializes REST fields from the
  **interface's** docblock, not the concrete class, so a wrong docblock
  type here silently corrupts `shipping_window` in both quote and order
  REST payloads (collapses to bare `true`/`false`).
- `shipping_window` is exposed over REST as a raw `ShippingWindowType` int
  with no label; the human-readable mapping only exists in
  `CheckoutConfig`'s `shipping_window_types` frontend config.

## Test environment specifics

- `dev/tests/integration`'s test database does **not** auto-apply
  new/changed data patches on a normal run (`TESTS_CLEANUP=disabled` means
  the existing installed DB just persists). If a data patch's logic changes
  after the DB was first installed, either run the
  `phpunit.xml.clear-database` variant for a full fresh reinstall, or
  manually delete the stale `patch_list` row (`patch_name` = the patch's
  fully-qualified class name, backslash-escaped) plus whatever rows that
  patch created, then re-run - `setup:upgrade` isn't directly invokable
  against this separate test DB via a plain `bin/magento` call.
- Chromedriver must match the installed Chrome version **exactly** or
  session creation fails immediately with a clear version-mismatch error.
  `npx chromedriver@<exact-version>` fetches a specific pinned version when
  the default/latest doesn't match.
- This workbench's Chrome may be a single persistent, shared instance (used
  by other tooling) rather than one chromedriver spawns fresh per session -
  under memory pressure this causes flaky hangs
  ("Timed out receiving message from renderer"). Force an isolated instance
  via `functional.suite.yml`'s `chromeOptions.args`: add `--headless=new`
  plus a dedicated `--user-data-dir=/tmp/<some-profile-dir>`. Clear/delete
  that profile directory between unrelated MFTF runs, or cart/session state
  (cookies, guest quotes) silently accumulates across runs (e.g. a cart
  quantity creeping up run after run) since the profile persists.
- `patches/*.patch.dev` files are applied by `vendor/bigbridge/patcher`
  (`Model/Patcher.php`) via `git apply --whitespace=nowarn
  --ignore-space-change --ignore-whitespace <patch>` on every `composer
  install`. Those flags only ignore whitespace *within* matched lines - a
  patch hunk whose context is missing/has an extra blank line compared to
  the real pristine vendor file still fails to apply, with no fuzz
  tolerance. This project's local dev vendor/ can already have a patch
  hand-applied from an earlier session, which hides a broken hunk context
  indefinitely (the file "looks patched" locally) until a genuinely fresh
  `composer install` - e.g. in CI - fails on it. To verify a `.patch.dev`
  file is actually valid, test it against a truly pristine copy of the
  target package, not local `vendor/`: build a throwaway composer project
  (`repositories` pointing at `https://repo.bigbridgedev.nl`, using
  `/opt/workbench/profile/bin/composer-auth` per the
  `workbench-composer-auth` skill, `--ignore-platform-reqs` if the PHP
  version here is newer than the package wants) requiring the exact locked
  version from this project's `composer.lock`, then run the same `git apply
  --whitespace=nowarn --ignore-space-change --ignore-whitespace` command
  against it.
