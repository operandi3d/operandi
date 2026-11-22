# Typography acceptance and unresolved distinctions

All corrected-application checks below are **Not run**. The headless font layout
experiment is reference evidence for metrics, not an application acceptance pass.

| ID | Setup and actions | Required result |
|---|---|---|
| FONT-T01 | Start the dedicated corrected macOS development bundle; inspect diagnostics for resolved family/face, requested size, font source, fallback and actual pixels-per-point. | Lucida Grande Regular candidate is used where intended; fallback is explicit. Record executable identity and active scale. No font asset is silently bundled. |
| FONT-T02 | At1x, measure Create, Polygon and Primitives in the shared egui font setup. Compare against the candidate measurements and Modo summaries in FINDINGS.md. | Reproduce candidate metrics within documented rounding tolerance, or explain a measured difference. Do not relabel advance agreement as perceptual sharpness parity. |
| FONT-T03 | Inspect ordinary menu/button, selection-mode, disclosure, Properties and numeric text at actual size on matched backgrounds. Include active/inactive, enabled/disabled, focus/error and long names. | Clear stems/counters and consistent spacing/weight; no clipped descenders, unintended bold, overlap or unreadable text. Existing expressions, field focus and command routes remain intact. |
| FONT-T04 | Inspect clockwise Create/Polygon labels at native1x and2x, then applicable1.25/1.5/scaled modes. Toggle categories, scroll properties, expand/collapse groups and resize. | Sharp centered labels, physical-scale-correct origins, no blurry resampled1x cache. Existing PAR-014a–d/T15–T17 dimensions, group rounding and top alignment remain intact. Distinguish simulated from physical checks. |
| FONT-T05 | Repeat with the runtime font unavailable through a controlled test fixture rather than deleting/renaming system fonts. | Explicit fallback, no crash or blank UI; labels fit or adapt deliberately without losing required controls. Record fallback differences. |
| FONT-T06 | Compare current and candidate typefaces with only that variable changed; subsequently vary size or rendering parameters separately if needed. | Evidence identifies what improves metrics and what improves actual-size clarity. Preserve approved palette and avoid changing multiple variables to manufacture a match. |
| FONT-T07 | Run relevant UI/layout/focus regressions and inspect native minimum/normal windows and supported display transitions. | Grid dimensions, picking boundaries, scroll reachability, selection, history and scene data remain correct. No Windows/Linux/physical-DPI claim from simulations alone. |

## Remaining questions

- **FONT-Q01:** Whether the typography candidate fully resolves the perceived
  sharpness difference requires a controlled corrected-app visual comparison.
  Font family/size mismatch is established; its share of the overall difference
  is not quantified by advance metrics alone.
- **FONT-Q02:** The SF font's default optical-size design may contribute, but
  the running font atlas's exact variation and coverage path were not traced.
  Do not assert optical size28 as a proven per-frame cause.
- **FONT-Q03:** Exact small-text equivalence across physical densities and future
  platforms remains untested. Keep those results separate from this macOS1x
  reference observation and the original1x/2x headless metrics.

Product decision needed: whether to adopt FONT-001–004 as the next bounded
macOS correction experiment. No further reference-product access is needed by
the implementer to run that experiment from these reviewed requirements.
