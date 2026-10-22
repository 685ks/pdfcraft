# Releasing PdfCraft

Releases are built by the shared Arthavan Projects release workflow,
[`685ks/craft-actions`](https://github.com/685ks/craft-actions) (`release-rust-app.yml`), which
`.github/workflows/release.yml` calls. It builds and packages PdfCraft for Windows
and creates the GitHub Release `v<version>` on this repository, as a draft unless `publish` is set.
The Arthavan Projects launcher installs and updates PdfCraft from these releases; started by it
(`ARTHAVAN_LAUNCHER=1`), PdfCraft runs no update check of its own, and About and *Help ▸ Check for
updates* say "Updates are managed by Arthavan Projects".

User-facing names say **PdfCraft**; files and binaries stay lowercase
(`pdfcraft-<version>-<platform>-<arch>.<ext>`), and the bundle id is `app.freedev.ainz.ks.PdfCraft`.
The ids, publisher (Arthavan Projects), categories and icons the workflow reads are in
[`packaging/app.toml`](../packaging/app.toml).

## Cutting a release

1. **Bump the version** on `main`. It lives only in `[workspace.package] version` in the root
   `Cargo.toml`:

   ```sh
   cargo xtask version                 # prints the current version
   cargo xtask version set 0.3.0       # or 0.3.0-rc.1; updates Cargo.toml and Cargo.lock
   ```

   Commit the change through the normal review flow.
2. **Run the workflow:** *Actions ▸ Release ▸ Run workflow*, with `version` (no leading `v`), and
   `publish` / `prerelease` as wanted. Without `publish` the release stays a draft.
3. **Check the draft** (download an installer or two), then publish it in the GitHub UI.

The packaging scripts below (`packaging/`) and their notes come from the upstream pipeline; they
still build the same packages locally and document what each one contains.

## What gets built

| Platform | Artifacts | Built on |
|---|---|---|
| Windows 10+ x64 | `pdfcraft-<v>-windows-x64.msi`, `pdfcraft-<v>-windows-x64-portable.zip` | `windows-latest` |
| Windows 10+ x86 (32-bit) | `pdfcraft-<v>-windows-x86.msi`, `pdfcraft-<v>-windows-x86-portable.zip` | `windows-latest` |
| Windows 11 on ARM64 | `pdfcraft-<v>-windows-arm64.msi`, `pdfcraft-<v>-windows-arm64-portable.zip` | `windows-latest` (cross-compiled) |
| Web | `pdfcraft-web-<v>.zip` (a static site; see [`packaging/web/README.md`](../packaging/web/README.md)) | `ubuntu-latest` |

The ARM64 Windows build is cross-compiled on the x64 runner, so signing and WiX work as for the
other Windows builds.

Every binary reports its version (`pdfcraft --version`, `pdfcraft-cli --version`, *Help ▸ About*).
The workflow sets `PDFCRAFT_BUILD_SHA` and `PDFCRAFT_BUILD_DATE` (`packaging/env.sh` fills them in
for local builds). The binaries don't embed the commit yet.

**Fonts:** every job checks out [craft-fonts](https://github.com/685ks/craft-fonts) at the commit
pinned in `release.yml` and builds with `CRAFT_FONTS_DIR` and `CRAFT_FONTS_REQUIRED=1`, so releases
embed its Japanese fonts and fail rather than ship without them (`AGENTS.md` §1.4). Bump the pin
deliberately.

**OCR models:** every desktop package ships the Scan & OCR models (#103). The packaging scripts call
`stage_models` (`packaging/env.sh`; `package.ps1` on Windows), which runs `cargo xtask models` to
fetch every `kind = "model"` file in `ATTRIBUTION.toml` (verified by SHA-256) with its licence text
and an `ATTRIBUTION.txt`, straight into the package, so packaging needs the network. They go where
`pdfcraft_ocr::Models::dirs_beside_exe` looks: `models\` beside `pdfcraft.exe` (MSI, portable zip). The web build doesn't include them yet.

### Windows

`packaging/windows/package.ps1 -Arch x64|x86|arm64` builds with `-C target-feature=+crt-static`, so
neither the MSI nor the portable zip needs the Visual C++ redistributable.

- Before packaging, it reads both executables' PE headers: the machine type must match `-Arch`, and
  `pdfcraft.exe` must be a GUI-subsystem program (no console window, #57) while `pdfcraft-cli.exe`
  stays a console program.
- `pdfcraft.wxs` (WiX v5) installs per machine into Program Files with a Start Menu shortcut, a
  desktop shortcut (on by default) and an App Paths entry. Both shortcuts are plain links to
  `pdfcraft.exe`, not advertised MSI shortcuts (#107, #143). The MSI version is the numeric `X.Y.Z`
  (MSI has no pre-release field), and same-version upgrades are allowed so release candidates replace
  each other. Icon ids end in `.ico` or `.exe` (Windows Installer requires it); packaging-lint checks
  this.
- `installer-ui.wxs` supplies native welcome, maintenance, progress, files-in-use and outcome
  dialogs. Full UI confirms success with Finish; failures and cancellations have distinct messages.
  The welcome dialog has a "Create a desktop shortcut" checkbox and an "Edit with PdfCraft"
  right-click menu checkbox, both ticked by default. `/qn` and `/qb` stay unattended and create
  both unless `INSTALLDESKTOPSHORTCUT=0` or `INSTALLPDFCONTEXTMENU=0` is passed;
  `INSTALLIMAGECONTEXTMENU=0` (no checkbox) leaves out the image menu. An update (the launcher runs
  `msiexec /passive` with no properties) keeps the desktop shortcut only if `PdfCraft.lnk` is still
  on the all-users desktop, so deleting it once keeps it away (#1058), and keeps each menu only if
  its key path is still in HKLM (#305). Both are AppSearch results read before the old version is
  removed. An update from a release that didn't have a menu yet (before 0.7.1 for the PDF menu,
  before 0.4.0 for the image menu, found by `OnlyDetect` Upgrade rows) adds it like a first install.
  A value given on the command line wins.
- The PDF right-click entry (#682) is a classic verb under
  `HKLM\Software\Classes\SystemFileAssociations\.pdf\shell\PdfCraft.Edit`, so it shows whichever app
  is the default for `.pdf` and never changes that default. Several selected PDFs (up to 15, the
  limit for a `Document` verb) start one `pdfcraft.exe "<file>"` each, and the single-instance
  hand-off opens them as tabs. On Windows 11 it is under "Show more options"; a first-level entry
  needs an `IExplorerCommand` handler and package identity (a sparse MSIX package), which the MSI
  doesn't have.
- The image right-click entry (#190) is the same kind of verb, `PdfCraft.CreatePdf`, for ten image
  types, in its own component (`PdfcraftImageContextMenu`) since it became optional. Its rows moved
  out of `PdfcraftApp`; that's safe because `MajorUpgrade` removes the old version before installing
  the new one (`afterInstallValidate`), which `test-msi.ps1` checks.
- `test-msi.ps1 <file.msi>` checks the compiled shortcut, context-menu and dialog tables, and
  evaluates the first-install and update conditions of the desktop shortcut and both menus. Packaging
  runs it in a child process before signing, so the MSI isn't held open when signtool runs. The ARM64 install
  smoke test checks that both all-users shortcuts point at the installed `pdfcraft.exe` and are
  removed on uninstall, and that `INSTALLDESKTOPSHORTCUT=0` skips the desktop one.
- The portable zip holds both executables, the README, the licences, the OFL licence of each
  embedded craft-fonts family, and `portable.txt`. That marker beside `pdfcraft.exe` keeps the
  settings, logs, crash recovery and new digital IDs in `PdfCraftData\` next to the exe instead of
  `%APPDATA%` and `%LOCALAPPDATA%` (`crates/ui-egui/src/portable.rs`).
- **Signing:** `packaging/windows/sign.ps1` signs both executables and the MSI with `signtool`
  (SHA-256, RFC 3161 timestamp), from a `.pfx` (`WINDOWS_CERTIFICATE`) or Azure Trusted Signing
  (`AZURE_*`), whichever is configured. It is the one place to change when signing changes.

Locally: `dotnet tool install -g wix --version 5.0.2`, then `pwsh packaging/windows/package.ps1 -Arch x64`.

### Web

`packaging/web/package.sh` runs `trunk build --release` in `apps/pdfcraft-web` and zips the site
with sample `_headers` and `.htaccess` files. Hosting (MIME types, compression, caching, iframes) is
covered in [`packaging/web/README.md`](../packaging/web/README.md).

## Secrets

The signing secrets live in the repository's **`release` environment** (*Settings ▸ Environments ▸
release*), restricted to the `release` branch; every job in `release.yml` declares
`environment: release`. Each secret is optional: if one is missing, that platform's artifacts are
unsigned and the run shows a warning.

| Secret | Used for |
|---|---|
| `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` | base64 Developer ID Application `.p12` and its password |
| `KEYCHAIN_PASSWORD` | the temporary CI keychain (random if unset) |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | `notarytool` (an app-specific password) |
| `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` | base64 `.pfx` code-signing certificate and its password |
| `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` | service principal for Azure Trusted Signing (instead of a `.pfx`) |
| `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE` | Trusted Signing endpoint, account and certificate profile |

`GITHUB_TOKEN` creates the release; only the final job gets `contents: write`.

## Checks

`.github/workflows/packaging-lint.yml` runs on changes to `packaging/` and the workflows: actionlint, shellcheck,
a PowerShell parse, xmllint (WiX) and
the MSI icon-id check. The Windows ARM64 workflow also exercise their packaging before a
release does.

## Scanner detections

Antivirus and reputation scanners sometimes flag a released file. Check the facts before you change
code or answer publicly. The first report was [issue #747](https://github.com/storytold/pdfcraft/issues/747),
a match of the `CobaltStrikeBeacon` family rule.

1. **Confirm the file is the release.** Compare its SHA-256 with `SHA256SUMS.txt` on the GitHub
   Release. Only a file whose hash matches the release says anything about that release.
2. **Record the signature.** `packaging/windows/security-audit.ps1` prints the Authenticode status:
   - `Valid`: the signature verifies.
   - `NotSigned`: no signature, as in developer and test builds. Not evidence of compromise by itself.
   - `HashMismatch`: the file changed after it was signed. Investigate.
   - `NotTrusted` or `UnknownError`: a signature exists but did not validate on this machine. Compare
     the signer and the hash with the release.
3. **Extract the payloads without running them.** Scan the executables inside the MSI and the
   portable zip, not only the outer installer. `7z x -oDIR pdfcraft-<version>-windows-x64.msi`
   extracts them. The payload names are WiX-internal, so tell the executables apart by their file
   type. Don't install or run the MSI or the extracted files.
4. **Scan with YARA and keep every match.** Use your own copies of YARA and the rule. PdfCraft
   bundles neither, and the helper never runs in the app, the installer or CI:

   ```powershell
   pwsh packaging/windows/security-audit.ps1 -Path .\pdfcraft.exe, .\pdfcraft-cli.exe -YaraExe C:\tools\yara64.exe -YaraRule C:\rules\CobaltStrikeBeacon.yar
   ```

   The output lists each matched identifier with its offset, and the SHA-256 of the rule file.
   Record where the rule came from and its revision. For CAPEv2's `CobaltStrikeBeacon` rule, that is
   the commit that last changed `data/yara/CAPE/CobaltStrikeBeacon.yar`.
5. **Find the condition branch.** Family rules usually have alternative branches, such as
   `all of ($x*) or 2 of ($a*) or ...`. Copy the rule, make one rule per branch with only the strings
   it uses, and scan again to see which branch fired.
6. **Interpret with care.** A match on generic repeated-byte patterns says much less than a match on
   distinctive code or strings, and it does not show that the software belongs to the family. Rules
   change too: CAPEv2 commit `9b599185` (2024-12-05) lengthened this rule's two repeated-byte patterns
   from 4 to 16 bytes to reduce false positives, so an older rule revision can match files that the
   current one does not. Don't alter PdfCraft's binaries to change a scanner's result. That changes
   nothing about the software.
7. **Report.** To the scanner, send the file's SHA-256, the release URL, the rule name and revision,
   and the matched identifiers and offsets. Report PdfCraft problems at
   <https://github.com/storytold/pdfcraft/issues> with the same facts.
