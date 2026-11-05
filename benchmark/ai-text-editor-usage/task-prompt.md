Edit task_source.py in the current directory. Make these three changes, in
any order you like:

1. Every call to the deprecated `legacy_log(...)` helper should be replaced
   with an equivalent `logger.info(...)` call, keeping the same message. Add
   `import logging` and `logger = logging.getLogger(__name__)` near the top
   of the file if they are not already present. You can leave the
   `legacy_log` function's own definition in place; just stop calling it.
2. Add a new function at the end of the file:
   `calculate_discount(price, percent)` that returns `price * (1 - percent / 100)`.
3. Add a one-line docstring to the `apply_tax` function explaining what it
   does, if it does not already have one.

Once those three changes are in task_source.py, do these two export steps,
in this order:

4. Write a new file, tax_rates.csv, in the same directory, with exactly two
   lines: the header `rate`, then one data line with the current numeric
   value of the `TAX_RATE` constant as it stands in task_source.py.
5. Write a second new file, departments.csv, in the same directory, with
   exactly this content:
   department,budget
   Sales,120000
   Support,80000
   Engineering,250000

Then run the shipped test file, test_task_source.py (`python3
test_task_source.py`), and make sure every test passes before you consider
the task done. If a test fails, figure out why and resolve it appropriately.

When you are done, print a short, one-line summary of what you changed and
stop.
