# M1 Polygon delivery entrypoint

The current implementation candidate is the working tree at `~/.codex/worktrees/m1-common-baselines/Operandi-Rust`, branch `codex/m1-common-baselines`. The approved [native QA app](</private/tmp/claude-501/-Users-jason-Documents-Operandi-Rust-bevel-weld-acceptance/28f88ed2-a492-4b5f-b111-13af9a2faa66/scratchpad/bundle-a1/Operandi Rust Bevel Weld QA.app>) (bundle ID `org.operandi.rust.bevelweldacceptance.qa`) was built from common-baselines; the clean QA reports and data are in `~/.codex/worktrees/671b/Operandi-Rust`. The director accepted the M1 Polygon delivery at its documented basic scopes, including the 11 tools, 16 command/display rows, six shared workflows and saved demonstrations. This technical acceptance does not claim user signoff, general reference-product parity or advanced fidelity. The work remains uncommitted and unpublished; the project license has not been chosen.

The 11 interactive Polygon tools and 16 command/display rows have implementations at their documented basic scopes. The three most recent tools—Chamfer Edit, Poly Haul and Sketch Extrude—use explicitly user-authorized **provisional Operandi** behavior. Their profiles, pointer mapping and gain are not claims about reference defaults. Unspecified modes, complex topology, mapped attributes, nonneutral contexts and other combinations may be refused; see the tool reports and demonstrations for the precise supported input. Evaluated subdivision, rendering, animation, rigging, simulation and sculpting are not established by this delivery.

## Open the demonstrations

The saved modeling packet (removed 2026-11-05) contains 20 native scenes, including Box → Bevel → Extrude → Smooth Shift → Spike, separate Thicken, Pen, Bridge and Reduction examples, and a tag-and-cleanup workflow. The three provisional tools packet (removed 2026-11-05) contains six more before/result scenes for Chamfer Edit, Poly Haul and Sketch Extrude. Both READMEs give selections, panel steps, supported limits and callable examples. Use **Save As** to keep your own variations without overwriting the supplied scenes. All 26 files were generated through current editor operations and native serialization, then reopened by the parser; scene files do not store the camera, so use **Frame All** after opening as needed. Automated generation is distinct from native interaction evidence.

## Current rows and default input map

The table records current Operandi command identities, original-artwork or text treatment, and the shipped default map—not the user's effective remaps or verified reference defaults. An em dash means unassigned in the current default map. `Alt+B` for Sketch Extrude is a documented **candidate** used provisionally. User remaps and conflict handling remain active. `tool.set <identity> on` activates each interactive tool. The table routes are not all complete parameterized invocations; required arguments and examples are in each tool’s admitted specification, demonstration README, and command help. See the [input map](../crates/operandi-app/src/input.rs), [command router](../crates/operandi-app/src/command.rs), [Polygon panel](../crates/operandi-app/src/toolbox.rs), and [artwork manifest](../crates/operandi-app/assets/icons/manifest.json) for the current implementation.

| ID | Interactive row | Tool identity | Treatment | Default |
|---|---|---|---|---|
| MP-T01 | Pen | `prim.pen` | Original icon | — |
| MP-T02 | Bevel | `poly.bevel` | Original icon | `Shift+B` |
| MP-T03 | Extrude | `poly.extrude` | Original icon | `Shift+X` |
| MP-T04 | Chamfer Edit | `chamfer.edit` | Original icon | — |
| MP-T05 | Smooth Shift | `poly.smshift` | Original icon | — |
| MP-T06 | Thicken | `Thicken` | Original icon | — |
| MP-T07 | Spike | `poly.spikey` | Original icon | — |
| MP-T08 | Poly Haul | `poly.haul` | Original icon | — |
| MP-T09 | Sketch Extrude | `poly.sweep` | Original icon | `Alt+B` candidate |
| MP-T10 | Bridge | `edge.bridge` | Original icon | — |
| MP-T11 | Polygon Reduction | `poly.reduct` | Original icon | — |

| ID | Command/display row | Current callable route | Treatment | Default |
|---|---|---|---|---|
| MP-C01 | Make | `poly.make auto` | Text | `P` |
| MP-C02 | Set Position | `vert.set` | Text | — |
| MP-C03 | Split | `poly.split` | Text | `Ctrl+L` |
| MP-C04 | Triple | `poly.triple` | Original icon | `Shift+T` |
| MP-C05 | Quadruple | `poly.quadruple` | Text | — |
| MP-C06 | Spin Quad | `poly.spinQuads` | Text | `Alt+V` |
| MP-C07 | Flip | `poly.flip` | Text | `F` |
| MP-C08 | Align | `poly.align` | Text | — |
| MP-C09 | Merge | `poly.merge` | Text | — |
| MP-C10 | Remove | `poly.remove` | Text | `Shift+Backspace` |
| MP-C11 | Collapse | `poly.collapse` | Text | `Ctrl+Backspace` |
| MP-C12 | Unify | `poly.unify` | Text | — |
| MP-C13 | Scale Offset | `@ppi_PolyScale_Offset.py` | Original shared script-action icon | — |
| MP-C14 | Flatten Ngons | `@ppi_flatten_ngons.py` | Original shared script-action icon | — |
| MP-C15 | Set Material | `poly.setMaterial` | Text | `M` |
| MP-C16 | Set Part | `poly.setPart` | Text | — |

The two script-named routes dispatch Operandi's bounded operations; they do not imply a general scripting host. The original milestone inventory (removed 2026-11-05) is a historical draft of minimums and is not a current implementation or reference-default catalog.

## Shared modeling roles

- **New Mesh:** creates and selects an empty mesh; its interactions with active tool ownership use the existing history policy. Default `N` is Scenes-scoped.
- **Mesh Cleanup:** the delivered basic cleanup removes unreferenced vertices; the other cleanup options are outside that basic operation.
- **Transform:** the combined component/item transform tool uses the shared action context and history.
- **Move**, **Rotate**, and **Scale:** component and item operations remain distinct; the current default bindings are `W`, `E`, and `R`. Context and topology restrictions remain those of the existing implementations.

The saved packet includes a New Mesh-based construction and a floating-vertex cleanup example. Earlier scoped component Transform/Move/Rotate/Scale evidence is reused; it does not establish all item-mode combinations or physical-display configurations.

## Validation and native evidence

The three provisional tools passed **14 focused automated tests** in total (4 Chamfer, 5 Poly Haul, 5 Sketch Extrude), with affected formatting, strict Clippy and application build checks passing. The one final workspace run returned **1,065 passed, 6 failed, 7 ignored**. The six presentation failures were repaired and each failed test passed individually afterward; the workspace suite was **not** run a second time, so the original full-suite result is not a clean pass. See the final regression status (removed 2026-11-05). The user confirmed the performance repair was greatly improved; no universal FPS claim follows.

The following are summaries of separately recorded native QA, reviewed by the director—not independent observations by the author of this entry:

- [Corrected three-tools pass](~/.codex/worktrees/671b/Operandi-Rust/docs/m1-three-tools-gui-20261030/corrected-pass-1/STATUS.md): Chamfer's specified 0.1 m / 2-segment / 50% result matched the admitted literal geometry and tags; one Sketch primary drag produced a selected cap and four connected walls while preserving the source section. The New Mesh command field produced the expected saved Scene.
- [New Mesh live-owner pass](~/.codex/worktrees/671b/Operandi-Rust/docs/m1-newmesh-owner-native-20261030/pass-1/STATUS.md): one live Smooth Shift → New Mesh → Undo case matched the corresponding admitted scene oracles within the reported tolerance.
- [New Mesh input routes](~/.codex/worktrees/671b/Operandi-Rust/docs/m1-newmesh-input-routes-20261030/pass-1/STATUS.md): the Scenes menu and Scenes-scoped `N` outputs matched the expected Scene exactly.
- [Shared basics pass](~/.codex/worktrees/671b/Operandi-Rust/docs/m1-shared-basics-20261030/final-pass-1/STATUS.md): floating-only Cleanup F01 saved the independently expected Scene, preserved the unselected item, and exercised native Undo/Redo and reopen. Item Move displayed offsets `(1, 2, 3)` in None/World context; the director reports its saved mesh unchanged. The report records the subsequent rotation/scale workflow as interrupted, with in-memory Undo/reopen equality not separately exported.

Extra focus, remap, checkbox and hardware/display-scale coverage remains qualified; these bounded checks are not a comprehensive native matrix. No additional GUI pass, installation, publication, license choice, or new reference investigation is implied by this entry.
