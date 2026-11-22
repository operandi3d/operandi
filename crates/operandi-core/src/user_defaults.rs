//! User preferences that core operations read when they make something new
//! (Preferences > Defaults > Application and Display > OpenGL): how a taken
//! name gets its number, the material and UV map names new geometry gets,
//! and a new Backdrop's sizing. The app sets them on its UI thread; each
//! thread starts with the factory values, so tests stay independent.
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// Item Index Style: how a number is added to a name that is taken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexStyle {
    /// `Item (2)`: Operandi's naming so far, so the provisional default.
    #[default]
    SpaceParentheses,
    /// `Item(2)`
    Parentheses,
    /// `Item 2`
    Space,
    /// `Item_2`
    Underscore,
    /// `Item2`
    Joined,
}
impl IndexStyle {
    pub const ALL: [Self; 5] = [
        Self::SpaceParentheses,
        Self::Parentheses,
        Self::Space,
        Self::Underscore,
        Self::Joined,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::SpaceParentheses => "Item (2)",
            Self::Parentheses => "Item(2)",
            Self::Space => "Item 2",
            Self::Underscore => "Item_2",
            Self::Joined => "Item2",
        }
    }
    /// The `n` for which `apply(base, n) == name`, if there is one.
    pub fn number(self, base: &str, name: &str) -> Option<u64> {
        let rest = name.strip_prefix(base)?;
        let digits = match self {
            Self::SpaceParentheses => rest.strip_prefix(" (")?.strip_suffix(')')?,
            Self::Parentheses => rest.strip_prefix('(')?.strip_suffix(')')?,
            Self::Space => rest.strip_prefix(' ')?,
            Self::Underscore => rest.strip_prefix('_')?,
            Self::Joined => rest,
        };
        // Only the spelling `apply` writes: no sign, no leading zeros.
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u64 = digits.parse().ok()?;
        (n.to_string() == digits).then_some(n)
    }
    /// `base` with the number `n` added in this style.
    pub fn apply(self, base: &str, n: u64) -> String {
        match self {
            Self::SpaceParentheses => format!("{base} ({n})"),
            Self::Parentheses => format!("{base}({n})"),
            Self::Space => format!("{base} {n}"),
            Self::Underscore => format!("{base}_{n}"),
            Self::Joined => format!("{base}{n}"),
        }
    }
}

/// Everything here, as one value the app sets.
#[derive(Clone, Debug, PartialEq)]
pub struct UserDefaults {
    pub index_style: IndexStyle,
    /// Default Material: the tag new primitives' polygons get. The factory
    /// `Default` means the default material, so they stay untagged.
    pub material: String,
    /// Default Texture Map: the name of the UV map primitives create.
    pub texture_map: String,
    /// New Backdrops size from their item's scale (Display Backdrops in
    /// Uniform Size) or from Pixel Size.
    pub backdrop_uniform_size: bool,
    pub backdrop_pixel_size: f64,
    /// Mesh Items: New Patch / New Catmull-Clark Display Level of new meshes.
    pub subdivision_level: u8,
    pub catmull_clark_level: u8,
    /// Auto Create UV Map: new mesh items start with an empty UV map named
    /// `texture_map`.
    pub auto_uv_map: bool,
    /// Auto Create Item Masks: new mesh items get a shader tree group that
    /// masks to them.
    pub auto_item_masks: bool,
    /// Default Primitive Polygon: Cone and Ellipsoid's starting Polygon type.
    pub primitive_polygon: crate::PolygonRepresentation,
    /// Symmetric Primitive Creation: with Symmetry on, primitives are made
    /// mirrored too.
    pub symmetric_primitives: bool,
    /// Copy and Paste selection rules.
    pub deselect_after_copy: bool,
    pub select_pasted: bool,
    pub deselect_before_paste: bool,
}
impl Default for UserDefaults {
    fn default() -> Self {
        Self {
            index_style: IndexStyle::default(),
            material: crate::material::DEFAULT_MATERIAL.into(),
            texture_map: crate::PRIMITIVE_UV_MAP.into(),
            backdrop_uniform_size: false,
            backdrop_pixel_size: crate::backdrop::DEFAULT_PIXEL_SIZE,
            subdivision_level: 2,
            catmull_clark_level: 2,
            auto_uv_map: false,
            auto_item_masks: false,
            primitive_polygon: crate::PolygonRepresentation::Face,
            symmetric_primitives: true,
            // Paste so far: the old selection cleared, the pasted elements
            // selected.
            deselect_after_copy: false,
            select_pasted: true,
            deselect_before_paste: true,
        }
    }
}

thread_local! {
    static CURRENT: RefCell<UserDefaults> = RefCell::new(UserDefaults::default());
}

/// The defaults this thread makes new things with.
pub fn current() -> UserDefaults {
    CURRENT.with(|c| c.borrow().clone())
}
/// Set this thread's defaults (the app's UI thread, at launch and when a
/// preference changes). Blank names fall back to the factory names.
pub fn set_current(mut defaults: UserDefaults) {
    let factory = UserDefaults::default();
    if defaults.material.trim().is_empty() {
        defaults.material = factory.material;
    }
    if defaults.texture_map.trim().is_empty() {
        defaults.texture_map = factory.texture_map;
    }
    if !(defaults.backdrop_pixel_size.is_finite() && defaults.backdrop_pixel_size > 0.) {
        defaults.backdrop_pixel_size = factory.backdrop_pixel_size;
    }
    defaults.subdivision_level = defaults
        .subdivision_level
        .min(crate::subdivision::MAX_LEVEL);
    defaults.catmull_clark_level = defaults
        .catmull_clark_level
        .min(crate::subdivision::MAX_LEVEL);
    if !matches!(
        defaults.primitive_polygon,
        crate::PolygonRepresentation::Face
            | crate::PolygonRepresentation::Subdivs
            | crate::PolygonRepresentation::CatmullClark
    ) {
        defaults.primitive_polygon = factory.primitive_polygon;
    }
    CURRENT.with(|c| *c.borrow_mut() = defaults);
}

/// The user's Item Index Style.
pub fn index_style() -> IndexStyle {
    CURRENT.with(|c| c.borrow().index_style)
}

/// `base` if `taken` says it is free, else `base` numbered from 2 in the
/// current Item Index Style.
pub fn free_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.into();
    }
    let style = CURRENT.with(|c| c.borrow().index_style);
    (2..)
        .map(|n| style.apply(base, n))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.into())
}

/// `free_name` with the taken names as a list, read once: the same answer,
/// but the cost of one pass over `names` however many numbers are taken.
pub fn free_name_among<'a>(base: &str, names: impl Iterator<Item = &'a str>) -> String {
    let style = CURRENT.with(|c| c.borrow().index_style);
    let mut base_taken = false;
    // taken[n] for the numbers 2.. that could be the first free one: a
    // scene holds at most MAX_SCENE_ITEMS names, so one of 2..=MAX + 2 is
    // free and bigger numbers never matter.
    let mut taken: Vec<bool> = Vec::new();
    for name in names {
        if name == base {
            base_taken = true;
        } else if let Some(n) = style.number(base, name)
            && n <= crate::MAX_SCENE_ITEMS as u64 + 2
        {
            let n = n as usize;
            if n >= taken.len() {
                taken.resize(n + 1, false);
            }
            taken[n] = true;
        }
    }
    if !base_taken {
        return base.into();
    }
    let n = (2..).find(|&n| !taken.get(n).copied().unwrap_or(false));
    style.apply(base, n.unwrap_or(2) as u64)
}

/// `symmetry`, or Off when Symmetric Primitive Creation is off.
pub fn primitive_symmetry(symmetry: crate::Symmetry) -> crate::Symmetry {
    if CURRENT.with(|c| c.borrow().symmetric_primitives) {
        symmetry
    } else {
        crate::Symmetry::default()
    }
}

/// The material tag new primitive polygons get: empty (the default
/// material) unless Default Material names another one.
pub fn new_polygon_material() -> String {
    let name = CURRENT.with(|c| c.borrow().material.clone());
    if crate::material::is_default_name(&name) {
        String::new()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_styles_number_taken_names() {
        let names = ["Mesh", "Mesh (2)"];
        let taken = |n: &str| names.contains(&n);
        assert_eq!(free_name("Cube", taken), "Cube");
        assert_eq!(free_name("Mesh", taken), "Mesh (3)");
        for (style, want) in [
            (IndexStyle::Parentheses, "Mesh(2)"),
            (IndexStyle::Space, "Mesh 2"),
            (IndexStyle::Underscore, "Mesh_2"),
            (IndexStyle::Joined, "Mesh2"),
        ] {
            set_current(UserDefaults {
                index_style: style,
                ..Default::default()
            });
            assert_eq!(free_name("Mesh", taken), want);
        }
        set_current(UserDefaults::default());
    }

    #[test]
    fn blank_names_fall_back_and_default_material_stays_untagged() {
        assert_eq!(new_polygon_material(), "");
        set_current(UserDefaults {
            material: "Metal".into(),
            texture_map: "  ".into(),
            backdrop_pixel_size: -1.,
            ..Default::default()
        });
        assert_eq!(new_polygon_material(), "Metal");
        assert_eq!(current().texture_map, "Texture");
        assert_eq!(current().backdrop_pixel_size, 0.01);
        set_current(UserDefaults::default());
    }
}
