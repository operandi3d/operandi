# Drop Action: open questions (build uses the pick until Jason answers)

- **DA-Q1 Persistence.** Does the choice survive restarting the app?
  Not measured. Pick: yes, saved in preferences; new installs start at (none).
- **DA-Q2 Place / Place and Align and the hierarchy.** One public page says
  Place also puts the source into the destination's hierarchy; the item-level
  pages and the documented "set an object on a surface" workflow only move it.
  Not scriptable to measure (needs a real mouse drop). Pick: transform only,
  no parenting. A one-drag hands-on check could settle it.
- **DA-Q3 Items list.** Public docs describe Drop Action for 3D viewports only.
  Pick: viewport only in v1; Items list keeps its normal reorder/parent drag.
- **DA-Q4 Multiple sources.** Not measured. Pick: apply to each selected item
  in selection order, one undo step total. Exception, the Insert family:
  only the pressed item is the source (inserting several items above one
  target has no obvious meaning).
- **DA-Q5 Insert position in the sibling list.** Measured: the source goes to
  the end of the destination's old parent's children, not into the
  destination's old slot. Sane alternative is the old slot. Pick: old slot
  (keeps the Items list from jumping); flag if Jason prefers the measured one.
- **DA-Q6 Place and Align heading.** Which way the item faces around the
  normal is not documented. Pick: the smallest rotation that takes the item's
  current world +Y onto the normal (keeps its heading as much as possible).
