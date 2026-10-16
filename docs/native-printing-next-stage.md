# Next stage: bounded native printing

This is a future implementation note; direct printing is not implemented by
0.39.0. Printable PDF layouts and manual Windows PDF-viewer printing already work.
Do not repeat the report renderer, branding, approval policy or financial logic.

Use existing validated report-presentation selection and the no-overwrite PDF
writer. Decide whether the first supported action opens an explicitly exported
PDF in an installed local viewer or submits to an explicit Windows print dialog;
do not call viewer launch proof of successful printing. Keep cancellation,
missing viewer/printer, rejected jobs and spooler failures distinct from success.
No automatic printing or automatic approval is authorized by selecting a report.

Before implementation, define the lifetime and private location of any generated
PDF, restart cleanup, races with lock/restore/client switches, path/association
handling and whether an external viewer can keep readable files after the app
locks. Prefer an explicit reviewed export and user-initiated action rather than
untracked temporary confidential files. Preserve scope, stale-context checks,
original identity/current warnings, original detailed exports and client-plan
privacy boundaries.

Test argument handling and file paths without invoking a shell, absent/locked/
foreign selections, cancellation and failed launches/jobs, no unexpected writes,
existing destinations and partial-file cleanup. Retain the complete native smoke
suite and real PDF page-content checks. Windows source compilation alone cannot
verify installed associations or printers. Record actual Windows dialog/job and
offline acceptance separately; keep the UI marked planned until the offered
behavior is implemented. Do not create an installer until the user requests one.
