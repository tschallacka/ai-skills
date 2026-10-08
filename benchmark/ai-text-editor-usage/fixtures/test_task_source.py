"""Regression tests for task_source.py.

Run after making changes: `python3 test_task_source.py`. All tests must
pass before you are done.
"""

from task_source import apply_tax, calculate_discount


def test_apply_tax_rate_is_seven_percent():
   result = apply_tax(100)
   assert abs(result - 107.0) < 1e-9, f"expected 107.0, got {result}"


def test_calculate_discount_basic():
   result = calculate_discount(200, 25)
   assert abs(result - 150.0) < 1e-9, f"expected 150.0, got {result}"


def main():
   tests = [test_apply_tax_rate_is_seven_percent, test_calculate_discount_basic]
   failures = 0
   for test in tests:
      try:
         test()
         print(f"PASS: {test.__name__}")
      except AssertionError as error:
         failures += 1
         print(f"FAIL: {test.__name__}: {error}")
      except Exception as error:  # noqa: BLE001 -- surface import/attribute errors as failures too
         failures += 1
         print(f"ERROR: {test.__name__}: {error}")
   if failures:
      print(f"{failures} test(s) failed")
      raise SystemExit(1)
   print("all tests passed")
   raise SystemExit(0)


if __name__ == "__main__":
   main()
