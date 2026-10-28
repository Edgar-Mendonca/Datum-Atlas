# Security

## Reporting a problem

Please report a security problem privately: use **Security › Report a vulnerability** on this repository (GitHub private advisory), or open an issue that only says you have something to report and we will contact you. Do not post details in a public issue.

## What Datum Atlas does and does not do

- Selections are `.datlas` files and your own materials are `.datdb` files (JSON), on your own computer.
- The only network call is the update check and download from this repository's GitHub Releases. Every download is checked against its SHA-256 checksum before it installs.
- The window can only open links to this repository (`open_url` refuses anything else).
- Files are treated as untrusted. The readers for projects, your materials, options and recovery files are tested with thousands of damaged files and must never crash the app.
- A damaged materials file is never written over: it is kept aside under a new name, and the app says so.
- Every file is saved through a temporary file and a rename, so a crash mid-save leaves the old file whole.
- The installer is not code-signed yet (planned for 1.0.0), so Windows shows a SmartScreen warning on the first install.
