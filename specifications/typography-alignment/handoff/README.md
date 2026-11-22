# Operandi typography alignment findings

Revision 1, 2026-10-21. Investigator-authored findings and a correction proposal
for Operandi - Rust Build. Read [FINDINGS.md](FINDINGS.md) and
[ACCEPTANCE.md](ACCEPTANCE.md). This package supplements the existing parity
and category-tab handoffs without changing their approval or implementation records.

Modo 17's queried application and visible-widget font is **Lucida Grande Regular,
10 points** at the observed 72 logical DPI and 1x display scale. Operandi's
current standard style loads the SF system font and uses size11. An isolated
egui 0.33.3 layout experiment brings several word advances much closer to the
Modo measurements by using Lucida Grande Regular at size10. This establishes a
font/metrics mismatch; it does not isolate every cause of perceived sharpness.

The proposed next correction is a controlled macOS font-matching experiment
before any rendering-backend change. It is not an approval record or a claim
that a corrected application has been built or visually accepted.

## Scope and evidence boundary

Included: three Markdown files and their manifest, containing original prose,
factual font/metric summaries, public technical citations and acceptance designs.
No font files, screenshots, application scripts, binary internals, raw widget
inventories or proprietary source are transferred. The author has prior
investigation-context exposure; no legal-clearance claim is made.

The private investigation used Modo's supported Run Script entry point for a
read-only query of public Qt font metadata. No scene commands, preference edits,
font replacements, event hooks or binary instrumentation were used. A separate
original headless research utility measured font layout using already-built
egui dependencies; it did not modify or rebuild Operandi. Private scripts and
raw outputs remain outside the implementation workspace.

## Review and delivery

Investigator self-review checked claim strength, stable IDs, arithmetic,
internal links, proposal status and evidence boundaries. This is not an
independent second review, user approval or application acceptance.
PACKAGE_MANIFEST.json records the listed files and hashes. All application
acceptance cases in this package remain Not run.
