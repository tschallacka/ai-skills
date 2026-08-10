# Testing companion: 02-step-append-button

## Browser verification
- Start from a fresh page with the one initial button.
- Direct input: mouse-click the rendered current last button once, then click each newly rendered current last button three more times.
- Pass: after clicks one, two, three, and four, the visible button count is respectively two, three, four, and five; generated buttons 1–4 are each immediately below the previous last button.
- Fail: any click appends zero or more than one button, appends elsewhere, or targets a non-last button.
- Isolated-proof status: not run; browser and HTML execution are prohibited by the benchmark.
