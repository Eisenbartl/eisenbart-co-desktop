# Windows evaluation guide

This development build provides a local workspace, client profiles, counseling
statuses, and dated session notes. Use fictional records only. Financial
calculations, statement imports, debt and savings plans, PDF reports, and
commercial licensing are still being developed. Password recovery and tested
backup/restore are not available.

## Install and open

1. Download the **windows-installer** ZIP from a successful Windows build. See
   the [Windows download instructions](windows.md).
2. Right-click the ZIP in File Explorer and choose **Extract All**. Open the
   extracted folder and read `README.txt`.
3. Open the installer `.exe` and follow its instructions. This development
   installer is unsigned, so Windows may show an unknown-publisher warning.
   Check that your download came from the expected repository build.
4. Open the Windows Start menu and search for **Eisenbart Counseling Suite**.
   Open that installed application.

## Create your workspace

On the first launch, enter **Organization name** and **Counselor name**. You may
replace the initial Eisenbart & Co. organization name. Choose these names
carefully; changing them later is not available in this build.

Enter a long, unique passphrase in **Workspace password**, with at least 12
characters, then enter it again in **Confirm password**. Retain it securely
outside the application. Choose **Create workspace** and wait for the dashboard.
There is no forgotten-password reset, and reinstalling cannot recover access.

## Add and update a fictional client

1. Choose **Add client**.
2. Enter **First name**, **Last name**, and **Household size**. Email, phone, and
   counseling goals are optional. Use invented details throughout evaluation.
3. Choose a **Counseling status**, then choose **Create client**. Wait for the
   client's name to appear in the directory and at the top of the saved record.
4. To edit a saved profile, choose **Clients** and select the client's name.
   Change the details or status, choose **Save changes**, and wait for
   **Client saved**.

In **Clients**, use **Search name, email, or phone** to find a record. The dropdown
beside it filters by counseling status; choose **All statuses** to remove that
filter. **Archived** records remain available in the directory. The dashboard
shows client counts and counseling stages; it does not calculate financial results.

## Save session notes

Open the intended client and enter text in **New session note**. Choose **Save
session note**, then confirm it appears in the history with the counselor's name
and date. A client must be created before notes can be added.

Save profile edits and session notes separately before switching records,
closing, or locking. If a prompt warns about unsaved changes, cancel it to return
and save your work. Saved notes remain in the history; add another note to record
a correction or update.

## Lock and return

Save your work, then choose **Lock workspace** when stepping away. Enter your
password and choose **Unlock workspace** to return. The application also locks
after 15 minutes of inactivity. That interval is fixed in this build. Automatic
locking clears unfinished edits and notes, so save frequently.

Close the application and reopen **Eisenbart Counseling Suite** from the Start
menu. Use **Unlock workspace** with the original password, then check that your
saved fictional client and notes are present. Records are kept on this computer
for the Windows user who created the workspace.

For offline evaluation, disconnect from the internet after installation and
repeat creating a fictional client, saving a note, locking, and reopening. Report
any failure; fresh Windows 11 offline acceptance remains a separate test.

## If something fails

Read the displayed message. For a password error, check your typing, Caps Lock,
and keyboard layout before trying again. For a loading error, use **Try again**,
**Retry loading clients**, or **Retry loading notes** when offered.

If the error continues, stop and report the exact message and what you were
doing. Keep your password private and preserve the local files. Do not delete,
edit, or reset the data folder, create a replacement workspace, or reinstall as
a recovery shortcut. The [Windows guide](windows.md) explains where files live;
the [security guide](security.md) explains password and recovery limits.
