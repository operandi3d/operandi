# Preferences: Navigation and input (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user. All on Input > Remapping.

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Mouse Input Preset | dropdown (native, plus presets copying other 3D apps) | not yet measured (the native preset expected) | Which mouse/keyboard mapping drives view navigation. |
| Input Device | dropdown: Mouse, Tablet, Hybrid | not yet measured | Mouse: drags keep going past the screen edge. Tablet: drags stop at the edge. Hybrid: Mouse behaviour for tool drags, Tablet for view navigation. Relative-mode tablets should use Mouse. |
| Allow Modifier Chording | checkbox | not yet measured | When on, modifier keys can take part in chords; when off, only mouse buttons can. |
| Numeric Enter as Tab | checkbox | not yet measured | Keypad Enter moves to the next field like Tab. |
| Open Dialogs under Mouse | checkbox | not yet measured | When on, dialogs open at the pointer; when off, where the last dialog was. |
| Tooltip Delay | seconds | 0.8 | Hover time before a tooltip appears. |
| Mouse Wheel Acceleration | percent | not yet measured | Wheel zoom speed; higher is faster. |
| Wheel Zoom at Mouse Cursor | checkbox | not yet measured | When on, wheel zoom centres on the pointer; when off, on the view centre. |
| Invert Mouse Wheel Zoom | checkbox | not yet measured | Reverses wheel zoom direction. |
| Reverse Pan Widget Direction | checkbox | not yet measured | Reverses the on-screen pan widget. |
| Zoom Drag Axis | dropdown: Left to Right, Right to Left, Bottom to Top, Top to Bottom | not yet measured | Drag direction that zooms in for drag-zoom. |
| Trackball Rotation | checkbox | off | Off: orbit turns around two axes only. On: free trackball orbit based on where the drag starts in the view. Each view can override it. |
| Override Viewport-Level Trackball Option | checkbox | not yet measured | When on, views can't override the trackball setting. |
| TrackBall Speed Multiplier (Mouse) | number | not yet measured | Trackball orbit speed with a mouse. |
| TrackBall Speed Multiplier (Tablet) | number | not yet measured | Same with a tablet. |
| Invert Drag Scrolling | checkbox | not yet measured | Reverses drag-panning in lists, browsers and forms. |
| Invert Mouse Wheel Scrolling | checkbox | not yet measured | Reverses wheel scrolling in lists, browsers and forms. |

Note: the reference shows a one-time prompt offering to try trackball
rotation after the first orbit; it disappears after 17 seconds. Optional.

Open: NAV-P-Q1 factory values; list of navigation presets.
