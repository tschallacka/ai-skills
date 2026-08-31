# Verification: 10-step-css-marker-form

## Automated tests

§ 2.1
Run the gate over the repository before and after the change and confirm the only difference is the newly readable stylesheets. Then prove the new form bites: a stylesheet with the marker removed fails, and one with the marker moved below the head-of-file window fails as unmarked. Confirm the three existing comment forms still classify their files, by checking one file of each kind.