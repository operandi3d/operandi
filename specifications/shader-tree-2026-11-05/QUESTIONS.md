# Shader Tree: open questions (build hand-off v0.2)

Everything here is **provisional**. Build with the stated placeholder; the
answer may change numbers or a rule later, rarely the structure.

| ID | Question | Placeholder for now | Matters for |
|---|---|---|---|
| Q-1b | Does a new scene already have an Environment item under Environments, and which lights appear under Lights? | Show whatever the scene has; no extra defaults. | Default scene |
| Q-2 | Where does Add Layer insert when a layer *inside* a group (not the group) is selected? | Inside that group, as first child. | Add Layer |
| Q-6 | Are Material, Part, Selection Set the only Polygon Tag Types? | Yes, those three. | Group properties |
| Q-7 | Which Material and Part tag does a brand-new mesh carry, and is the name "Default" special? | Both "Default"; Base Material covers everything. | Mesh defaults |
| Q-8 | Apply to Sub-Group: exact reach (union of descendant group masks? nested levels?). | Store the flag only. | Evaluator |
| Q-9 | Layer Mask direction for **light and environment** textures. Surfaces: **answered** (masks only the one layer directly above, §8 item 4). | Same as surfaces. | Evaluator |
| Q-10 | Where can a Group Mask layer sit in its group? Two Group Masks: **answered**, they combine by their own blend modes (§8 item 6). | Anywhere. | Evaluator |
| Q-11 | Do mask layers use their own opacity, invert and blend? Can a mask have a mask? | Yes; no. | Evaluator |
| Q-12 | What Normal Multiply and Normal Map Blend compute. | Operandi defines its own. | Evaluator |
| Q-13 | **Answered** (§8 item 7): folded first, then the whole channel set blended with the group's mode and opacity. | — | Evaluator |
| Q-14 | **Answered**: topmost applicable shader wins (§8 item 8). Shader opacity/blend below 100 % not tested. | Topmost wins. | Evaluator |
| Q-15 | **Answered** (§8 item 5): starts from the result below the group. | — | Evaluator |
| Q-16 | Two materials stacked at partial opacity: per-channel mix or BRDF mix? | Per-channel mix. | Evaluator |
| Q-17 | Texture alpha × opacity × layer mask = one coverage? | Yes. | Evaluator |
| Q-18 | What Invert on a Material Group inverts. Measured: no visible effect on a group with one Normal colour layer. | Store only. | Evaluator |
| Q-19 | Bump stacking and bump + normal map together. | Later. | Evaluator |
| Q-20 | Displacement clipping range. | Later. | Evaluator |
| Q-22 | Default of Apply to Item Instances; how instances match an item mask. | Default on; match by item identity. | Group properties |
| Q-23 | Default Scope. | Surface. | Group properties |
| Q-24 | Exact Add Layer menu and Effect menu order per item type. | Order in §5.1 / §7 of the behaviour file. | Menus |
| Q-25 | Scene-file storage of the tree (order, masks, blend, effects). | Operandi's own format first; LXO mapping later. | Save/load |
| Q-26 | Do items placed above the Base Shader at the root shade anything (outputs aside)? | Allow any order; evaluator decides later. | Ordering |
| Q-27 | Is Exclusion also a blend mode? | No (16 modes). | Blend menu |
| Q-28 | Default projection of a new procedural (Solid?) and of a new image (UV Map when the mesh has UVs?). | Procedural: Solid. Image: UV Map. | Texture defaults |
| Q-29 | What the Effect column shows for a Material Group row (Base Material shows "(all)"). | The group's mask, e.g. tag name, else "(all)". | Panel |
| Q-30 | Does the B column show anything for Normal? | Blank for Normal, mode name otherwise. | Panel |
| Q-42 | Can Material, Shader or Render Output rows have children (e.g. a layer mask)? | No. | Parenting |
| Q-43 | Item mask naming a group of items: does it cover all members, including ones added later? | Yes, all current members, live. | Group properties |
| Q-44 | Exact Library guard behaviour for masked groups (refuse, warn, auto-wrap). | Refuse with a message. | Library drop |
