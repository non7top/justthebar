# Install and update

Download `jtb-<version>-windows-x64-setup.exe` from the release page and run it. It installs for the
current user to `%LOCALAPPDATA%\Programs\JustTheBar`, with no admin rights, and adds a Start menu
shortcut and an Apps entry that uninstalls it.

## Updating

Run the newer setup over the old one. If a bar is running, the installer quits it (`jtb-app.exe --quit`),
replaces the files and, on the last page, offers "Restart JustTheBar". A silent update
(`setup.exe /S`) restarts the bar by itself when it was running. `jtb.json` and the logs are kept.

`jtb-app.exe --version` prints the installed version. The zip holds the same files without an
installer, for running the bar from any folder.

## Verifying a download

Each release attaches `SHA256SUMS` and, for `jtb-app.exe`, `jtb_hook.dll`, the zip, the installer
and `SHA256SUMS`, two Sigstore records made by the release workflow with no key held:

- a GitHub build provenance attestation (checked with `gh`, or with the sigstore-shell Explorer tab, which reads the `ProvenanceRepo` claim embedded in the files)
- a cosign keyless signature bundle, `<file>.cosign.bundle`

```
gh attestation verify jtb-<version>-windows-x64-setup.exe --repo non7top/justthebar
cosign verify-blob --bundle jtb-<version>-windows-x64-setup.exe.cosign.bundle --certificate-identity-regexp '^https://github.com/non7top/justthebar/' --certificate-oidc-issuer https://token.actions.githubusercontent.com jtb-<version>-windows-x64-setup.exe
```

The files are not Authenticode signed, so Windows SmartScreen may warn on the first run.
