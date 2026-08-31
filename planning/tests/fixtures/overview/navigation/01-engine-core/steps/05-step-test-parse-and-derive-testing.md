# Verification: 05-step-test-parse-and-derive

## Automated tests

§ 2.1
Invoke W102's production extractor and serialize its canonical output, parse it through W02, and compare every emitted field and value. Then add an unknown field and truncate the serialized output, asserting the named failures. This is the production round-trip proof, not only a parser-fixture test.
