# Windows Exam Shell Mode (Operational Hardening)

This adds OS-level containment for a dedicated exam account, so Ctrl+Alt+Del escape paths are operationally reduced.

## What this does

- Creates or reuses a local exam account (default: `RestrictedExam`).
- Sets Restricted IDE as the per-user shell for that exam account.
- Applies per-user policies to reduce escape routes:
  - Task Manager disabled
  - Lock workstation disabled
  - Change password disabled
  - Logoff / Run / Control Panel / Win keys disabled for exam account
- Optional machine-wide policy:
  - Hide fast user switching

## What this does not guarantee

- Windows secure attention sequence (`Ctrl+Alt+Del`) cannot be fully blocked from user mode.
- This script reduces practical abuse, but does not provide kernel-level anti-cheat.

## Prerequisites

- Run scripts from an Administrator PowerShell session.
- Use packaged executable path for `AppPath`.
- Exam user profile must exist once (`C:\Users\<ExamUser>\NTUSER.DAT`).
  - If missing: sign into that user once, then sign out.

## Enable

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\enable-exam-shell.ps1 -AppPath "C:\Program Files\RestrictedIDE\restricted-ide.exe" -ExamUser "RestrictedExam"
```

Optional machine-wide policy:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\enable-exam-shell.ps1 -AppPath "C:\Program Files\RestrictedIDE\restricted-ide.exe" -ExamUser "RestrictedExam" -ApplyMachineWidePolicies
```

## Disable / Rollback

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\disable-exam-shell.ps1 -ExamUser "RestrictedExam"
```

Optional rollback of machine-wide policy:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\disable-exam-shell.ps1 -ExamUser "RestrictedExam" -RemoveMachineWidePolicies
```

## Recommended operational flow

1. Admin enables exam-shell setup once.
2. Students sign in with the dedicated exam user only during tests.
3. App-level kiosk + strict/ultra mode enforces session runtime controls.
4. After exams, admin runs disable script.

## Safety notes

- Always keep at least one separate admin account for emergency recovery.
- Never test first on your primary daily profile.
- If shell lockout occurs, sign in as admin and run disable script.
