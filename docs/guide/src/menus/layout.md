<!-- help-ids: menu.layout ; template: panel ; since: 0.1.0 -->
<!-- Written by crates/operandi-app/src/commands/guide_menus_tests.rs from the app's menus. Don't edit by hand: run OPERANDI_BLESS_GUIDE=1 scripts/check-fast.sh guide_menus -->
# Layout menu

The entries of the **Layout** menu, in order. A **▸** opens a submenu. Keys in brackets run the entry; the `code` is the command it runs.

- **New Layout…** · `operandi.layout.create`
- **Save Layout** · `operandi.layout.save`
- **Save Layout As…** · `operandi.layout.saveAs`
- **Rename Layout…** · `operandi.layout.rename`
- **Delete Layout…** · `operandi.layout.delete ask:true`
- **Reset Layout…** · `operandi.layout.revert ask:true`
- **Clear Layout** · `operandi.layout.clear`
- **Layouts ▸**
  - **Model** · `layout.restore Model`
  - **UV** · `layout.restore UV`
  - **Animate** · `layout.restore Animate`
- **Recent Layouts…** · `operandi.layout.switcher`
- **Lock Divider Dragging** · `layout.lockDividers`
- **UI Build Mode** (**Control+Shift+L**) · `operandi.layout.buildMode`
- **Windows/Palettes ▸**
  - **Toolbox** · `layout.window hash:toolbox`
  - **Items** (**5**) · `layout.window hash:items`
  - **Properties** · `layout.window hash:properties`
  - **Shading** · `layout.window hash:shading`
  - **Groups** · `layout.window hash:groups`
  - **Vertex Maps** · `layout.window hash:vertexMaps`
  - **Command History** · `layout.window hash:commandHistory`
  - **Preset Browser** · `layout.window hash:presetBrowser`
  - **UV Editor** · `layout.window hash:uvEditor`
  - **UV View** · `layout.window hash:uvView`
  - **Form Editor** · `layout.window hash:formEditor`
  - **Vertex Normal Tools** · `layout.window hash:vertexNormalTools`
  - **Statistics** · `layout.window hash:statistics`
  - **Node View** (**Ctrl+Shift+N**) · `operandi.window.nodeView`
  - **Toggle Palettes** (**` (backtick)**) · `operandi.layout.togglePalettes`
- **Workbench** · `operandi.toolbox.tab workbench`
- **Top Workbench** · `operandi.workbench.topBar`

## Related

- [Menus](index.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
