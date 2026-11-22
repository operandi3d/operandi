# Layouts stage 7: menu editing and the Form Editor

Behaviour only.

Stage 7. Menus and toolbars become forms; System > Form Editor edits any
form. Rule landing: UIC-213; UIC-10's "form editor is future work" ends.

- **UIC-10, UIC-210:** Menus and toolbars become forms too. Convert the menu
  bar one menu at a time, with a paint/behaviour-identical check per menu
  before deleting old code. If a menu is hairy, leave it hard-coded, log it,
  and ship the rest.
- **LAY-83:** System > Form Editor. On the left, a tree of groups > forms >
  controls, with a search field and Find Form (click any part of the UI to
  jump to its form). Groups are only for tidiness. On the right, the
  selected form's or control's properties (the same ones as the workbench
  right-click). New group, new form, add a command, add a sub-form, drag to
  reorder, Preview Form, and Save Form to its own file. Commands can be
  taken from the command history (UIC-160).
- **LAY-Q53:** The "Customizable Workbench" setting becomes a checkbox in the
  Form Editor (it was a form-file flag). Show in Form Editor on the
  workbench right-click menu gets un-greyed.
- **LAY-87:** User edits sit over shipped forms, with Revert per form.
  Head/tail slots and show-only-when (UIC-214, UIC-215) appear as
  properties.
- **UIC-183:** The Form Editor is a floating window.
