# Testing companion: 03-step-completion-branch

## Browser verification
- Continue the same bounded flow from the five-button state where generated button 4 is current last.
- Direct input: mouse-click generated button 4 once.
- Pass: the document is cleared of buttons and the only completion content is exact lowercase `finished`.
- Fail: pressing generated button 4 appends another button, leaves prior buttons, changes case, or emits extra completion text.
- Isolated-proof status: not run; browser and HTML execution are prohibited by the benchmark.
