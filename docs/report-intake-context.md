# Selected intake in internal reports — 0.26.0 source

Saved counselor assessments can optionally include mailing address, household
composition and/or employment/income context. Nothing is selected automatically.
Intake remains counselor-entered context, separate from approved transactions,
verified financial facts and financial-model inputs. Empty selected fields show
**Not recorded**. Follow-up tasks and their outcome notes are excluded.

## Use it

1. Save the client's intake in **Intake and follow-ups**.
2. Open **Saved counselor assessments**, enter the period and report details,
   and enable **Include selected saved intake context in this internal report**.
3. Choose **Review saved intake**, then select only the sections needed. Review
   the visible selected values and confirm the assessment acknowledgment again.
4. Save the record and export its internal assessment PDF. It retains the intake
   version and original selected values. Client action-plan PDFs omit this context.

A changed intake version between preview and save blocks the save without losing
the draft. Review again and acknowledge explicitly; the application does not
retry a report write automatically. An unsaved intake cannot be included.

Later changes to selected values permanently flag the report **Needs review**,
including when those values are subsequently reverted. The assessment directory
and opened record use the same selected-field history check. The original report and
its digest remain unchanged. An unselected address change or unrelated follow-up
edit does not flag intake context. Other existing source/profile/model review
rules continue to apply. Client agreements inherit supporting-report review flags.

## Persistence and validation

Rust authorizes the scoped client and captures the selected original values in
the existing atomic report/audit transaction. UI input supplies only an expected
care revision and section names. Reads and encrypted recovery compare captured
values/timestamp to the original scoped append-only care version, rejecting even
a report altered and rehashed to contain different intake. Later bounded care
history detects changes to selected fields without converting notes into facts.

Schema 17 and dependencies are unchanged. Reports with intake use payload format
5, optionally alongside session evidence or captured/saved planning scenarios.
Reports without intake retain formats 1–4; absent optional intake is omitted from
serialization, preserving their original bytes/digests. Older applications do
not understand format 5: do not reopen an updated workspace in an older build.
There is no new installer for this source checkpoint.

This remains an incomplete internal assessment workflow. Final report acceptance,
organization logo/contact branding, remaining model/document requirements and
commercial release verification remain separate work.
