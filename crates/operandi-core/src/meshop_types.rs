//! The MeshOp type table (KIT-1, kits plan K7; AR-09): one entry per op
//! type, shared by Operandi's own ops and ops that kits add.
//!
//! An entry ([`OpDef`]) says everything the stack, the file reader and the
//! Properties panel need about an op type: its saved type id, label and
//! browser folder, whether it has a Selection input, its values with their
//! form fields (KIT-4: kits describe, the app draws), the other items it
//! reads (item links, SAFE-07..09) and how to run it. Adding an op type is
//! one module plus one entry here; nothing else in the stack changes.
//!
//! Most ops are written as a plain value struct with a `run` method
//! ([`TypedOp`]) and wrapped by [`Typed`], which turns the saved JSON into
//! the struct and back, so unknown names and wrong types are refused in one
//! place (SAFE-10).
//!
//! The table is process-wide: a scene file's ops are read before any
//! editor exists. Built-ins come first, in Add Operator order; kits append
//! with [`register`]. An op whose type isn't in the table yet loads as a
//! placeholder that keeps its data (SAFE-12) and runs as soon as a kit
//! registers its type.
use crate::{Mesh, Result, Shared};
use glam::DMat4;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock, RwLock};

pub type Text = Cow<'static, str>;

/// How a value shows in the Properties panel and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// Metres.
    Distance,
    /// Stored in radians, shown in degrees.
    Angle,
    /// Stored as a fraction (1.0 = 100%), shown as a percentage.
    Percent,
    /// A plain number with no unit.
    Number,
    /// A whole number in `min..=max`.
    Count {
        min: u32,
        max: u32,
    },
    /// An index into these names.
    Choice(Vec<Text>),
    Check,
    Text,
    /// A link to another mesh item by its id; 0 = none (SAFE-08).
    Item,
    /// Links to several mesh items, as a list of ids.
    Items,
    /// Saved and loaded as it is, never shown (a value only files use).
    Hidden,
}

/// One value of an op: its saved name (the reference attribute name, so
/// `item.channel <key> <value>` ports), its label and its kind.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub key: Text,
    pub label: Text,
    pub kind: FieldKind,
}
impl Field {
    pub fn new(key: &'static str, label: &'static str, kind: FieldKind) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind,
        }
    }
}

/// Another item's mesh as an op sees it: the item's shown result (its own
/// stack evaluated first) and where it and the reading item sit.
#[derive(Clone, Debug)]
pub struct Source {
    pub item: u64,
    pub name: String,
    pub mesh: Shared<Mesh>,
    /// Source local → this item's local: where it appears in the viewport
    /// relative to the reading item.
    pub to_local: DMat4,
    /// Source local → world.
    pub world: DMat4,
    /// Source local → this item's local in the source's setup (rest)
    /// pose (ACS-API-31): the skin's bind pose (skin.rs).
    pub rest: DMat4,
}
impl Source {
    /// The source's mesh moved into the reading item's space. Vertex
    /// normals and morph offsets turn with it, and a mirroring transform
    /// keeps the polygons facing out.
    pub fn local_mesh(&self) -> Mesh {
        self.placed_mesh(self.to_local)
    }
    /// The source's mesh moved by `m` (e.g. [`Source::world`] for an op
    /// that places sources by their world transform alone).
    pub fn placed_mesh(&self, m: DMat4) -> Mesh {
        if m == DMat4::IDENTITY {
            return self.mesh.to_owned_value();
        }
        crate::duplicate::transformed_copy(&self.mesh, m, false)
    }
}

/// One item link of an op, resolved: the source, or why there is none
/// ("missing input", "dependency loop"; SAFE-07, SAFE-08).
#[derive(Clone, Debug)]
pub struct Linked {
    pub item: u64,
    pub source: std::result::Result<Source, String>,
}
impl Linked {
    /// Same inputs: the same items with the same results at the same
    /// place. Equal links let a cached result stand (MO-03).
    pub fn same(a: &[Linked], b: &[Linked]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(a, b)| {
                a.item == b.item
                    && match (&a.source, &b.source) {
                        (Ok(x), Ok(y)) => {
                            Shared::same(&x.mesh, &y.mesh)
                                && x.to_local == y.to_local
                                && x.world == y.world
                                && x.rest == y.rest
                        }
                        (Err(x), Err(y)) => x == y,
                        _ => false,
                    }
            })
    }
}

/// What an op reads when it runs.
pub struct OpContext<'a> {
    /// The result of the op below (or the Base Mesh).
    pub input: &'a Mesh,
    /// This op's id in its stack (it names the groups it writes).
    pub op: u64,
    /// The polygons its Selection input picked; `None` = the whole mesh
    /// (MO-20, PG-05). Already checked against `input`.
    pub selection: Option<&'a [u32]>,
    /// The op's item links, in the order [`OpDef::links`] gave them.
    pub links: &'a [Linked],
    /// The scene time the stack is evaluated at (AR-02). Only ops that
    /// say [`OpDef::uses_time`] are re-run when it changes.
    pub time: crate::time_tick::Tick,
}
impl OpContext<'_> {
    /// The chosen polygons, or empty for the whole mesh.
    pub fn chosen(&self) -> Result<Vec<u32>> {
        Ok(self.selection.map(<[u32]>::to_vec).unwrap_or_default())
    }
    /// The chosen polygons, or every polygon.
    pub fn chosen_or_all(&self) -> Result<Vec<u32>> {
        Ok(match self.selection {
            Some(s) => s.to_vec(),
            None => (0..self.input.faces.len() as u32).collect(),
        })
    }
    /// Records group `group` (a short id, PG-03) of `mesh`, this op's
    /// result, for Select by Previous Operation above it.
    pub fn tag(&self, mesh: &mut Mesh, group: &str, faces: impl IntoIterator<Item = u32>) {
        let set: std::collections::BTreeSet<u32> = faces
            .into_iter()
            .filter(|&f| (f as usize) < mesh.faces.len())
            .collect();
        mesh.attributes
            .polygon_sets
            .insert(crate::meshop_selection::group_set(self.op, group), set);
    }
    /// Every linked source that resolved; the first failure fails the op
    /// with its reason ("missing input" passes the op through, SAFE-08).
    pub fn sources(&self) -> Result<Vec<&Source>> {
        self.links
            .iter()
            .map(|l| l.source.as_ref().map_err(Clone::clone))
            .collect()
    }
}

/// One op type (KIT-1, K7).
pub trait OpDef: Send + Sync {
    /// The saved type id: what files store and `meshop.create` takes.
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    /// The Add Operator folder under Mesh Operations (MO-17, AB §5),
    /// subfolders split by `/`: `Polygon`, `Create/Primitives`.
    fn folder(&self) -> &str;
    /// Whether the op has a Selection input (MO-13).
    fn takes_selection(&self) -> bool;
    /// The values' saved version (SAFE-12). Bump when a value changes
    /// meaning and upgrade older ones in [`OpDef::read`].
    fn version(&self) -> u32 {
        1
    }
    /// The Properties form, in order. Every saved value has a field.
    fn fields(&self) -> &[Field];
    /// Every value at its default (MO-40: the direct tool's defaults).
    fn defaults(&self) -> serde_json::Value;
    /// Values saved at `version`, checked and upgraded to this version, or
    /// why they don't fit (the op then loads as a placeholder).
    fn read(&self, version: u32, values: &serde_json::Value) -> Result<serde_json::Value>;
    /// The items this op reads (SAFE-07..09). By default every `Item` and
    /// `Items` field; 0 and missing mean "none".
    /// `scene` lets a type expand a link, e.g. a group to its meshes or a
    /// driver to its children; it must not read anything else.
    fn links(&self, values: &serde_json::Value, scene: &crate::Scene) -> Vec<u64> {
        let _ = scene;
        field_links(self.fields(), values)
    }
    /// A new op's values, given the other items selected when it was
    /// added (in selection order). Boolean takes the first as its driver
    /// (BOP-10); most ops ignore them.
    fn created(&self, values: &mut serde_json::Value, picked: &[u64]) {
        let _ = (values, picked);
    }
    /// The groups its result offers Select by Previous Operation, as
    /// (short id, label) (PG-03: the id is saved, the label shown).
    fn groups(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Whether the result depends on the scene time.
    fn uses_time(&self) -> bool {
        false
    }
    /// The op on `ctx.input`. Values were read by [`OpDef::read`]; still
    /// re-check anything a script could have set out of range (SAFE-10).
    fn run(&self, ctx: &OpContext, values: &serde_json::Value) -> Result<Mesh>;
}

/// The item ids held by a form's `Item` and `Items` fields.
pub fn field_links(fields: &[Field], values: &serde_json::Value) -> Vec<u64> {
    let mut out = Vec::new();
    for f in fields {
        let v = &values[f.key.as_ref()];
        match f.kind {
            FieldKind::Item => out.extend(v.as_u64().filter(|&id| id != 0)),
            FieldKind::Items => out.extend(
                v.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|x| x.as_u64())
                    .filter(|&id| id != 0),
            ),
            _ => {}
        }
    }
    out
}

/// An op written as a value struct (serde names = saved names).
pub trait TypedOp: Serialize + DeserializeOwned + Default + Send + Sync + 'static {
    fn run(&self, ctx: &OpContext) -> Result<Mesh>;
}

/// The table entry for a [`TypedOp`].
pub struct Typed<T> {
    pub id: &'static str,
    pub label: &'static str,
    pub folder: &'static str,
    pub takes_selection: bool,
    pub fields: Vec<Field>,
    pub time: bool,
    /// Select by Previous Operation groups, (id, label).
    pub groups: Vec<(&'static str, &'static str)>,
    pub _values: PhantomData<fn() -> T>,
}
impl<T: TypedOp> Typed<T> {
    pub fn new(
        id: &'static str,
        label: &'static str,
        folder: &'static str,
        takes_selection: bool,
        fields: Vec<Field>,
    ) -> Self {
        Self {
            id,
            label,
            folder,
            takes_selection,
            fields,
            time: false,
            groups: Vec::new(),
            _values: PhantomData,
        }
    }
    /// The same entry offering these (id, label) groups.
    pub fn with_groups(mut self, groups: &[(&'static str, &'static str)]) -> Self {
        self.groups = groups.to_vec();
        self
    }
    fn parse(values: &serde_json::Value) -> Result<T> {
        serde_json::from_value(values.clone()).map_err(|e| e.to_string())
    }
}
impl<T: TypedOp> OpDef for Typed<T> {
    fn id(&self) -> &str {
        self.id
    }
    fn label(&self) -> &str {
        self.label
    }
    fn folder(&self) -> &str {
        self.folder
    }
    fn takes_selection(&self) -> bool {
        self.takes_selection
    }
    fn fields(&self) -> &[Field] {
        &self.fields
    }
    fn defaults(&self) -> serde_json::Value {
        serde_json::to_value(T::default()).unwrap_or(serde_json::Value::Null)
    }
    fn read(&self, version: u32, values: &serde_json::Value) -> Result<serde_json::Value> {
        if version != self.version() {
            return Err(format!(
                "Saved by a newer Operandi (version {version} of this operation)"
            ));
        }
        let typed = Self::parse(values)?;
        serde_json::to_value(typed).map_err(|e| e.to_string())
    }
    fn uses_time(&self) -> bool {
        self.time
    }
    fn groups(&self) -> Vec<(String, String)> {
        self.groups
            .iter()
            .map(|(id, label)| ((*id).into(), (*label).into()))
            .collect()
    }
    fn run(&self, ctx: &OpContext, values: &serde_json::Value) -> Result<Mesh> {
        Self::parse(values)?.run(ctx)
    }
}

static TABLE: LazyLock<RwLock<Vec<Arc<dyn OpDef>>>> =
    LazyLock::new(|| RwLock::new(crate::mesh_stack_builtin::builtin()));

fn table() -> std::sync::RwLockReadGuard<'static, Vec<Arc<dyn OpDef>>> {
    TABLE.read().unwrap_or_else(|e| e.into_inner())
}

/// The op type `id`, if this build or a loaded kit knows it.
pub fn op_def(id: &str) -> Option<Arc<dyn OpDef>> {
    table().iter().find(|d| d.id() == id).cloned()
}

/// Every op type, in Add Operator order: built-ins, then kits' in the
/// order they registered.
pub fn op_defs() -> Vec<Arc<dyn OpDef>> {
    table().clone()
}

/// Adds a kit's op type (K7). The id must be new and dotted like the
/// built-ins (`mykit.cable.item`); every saved value needs a form field so
/// the Properties panel can show it (KIT-4).
pub fn register(def: Arc<dyn OpDef>) -> Result<()> {
    let id = def.id().to_string();
    if !id.contains('.') || id.starts_with('.') || id.ends_with('.') || id.contains(' ') {
        return Err(format!(
            "A mesh operation type needs a dotted id like “kit.name.item”, not “{id}”"
        ));
    }
    let defaults = def.defaults();
    let map = defaults
        .as_object()
        .ok_or_else(|| format!("{id}: its default values must be an object of named values"))?;
    for key in map.keys() {
        if !def.fields().iter().any(|f| f.key == key.as_str()) {
            return Err(format!("{id}: the value “{key}” has no form field"));
        }
    }
    def.read(def.version(), &defaults)
        .map_err(|e| format!("{id}: its own defaults don't read back: {e}"))?;
    let mut t = TABLE.write().unwrap_or_else(|e| e.into_inner());
    if t.iter().any(|d| d.id() == id) {
        return Err(format!("The mesh operation type “{id}” already exists"));
    }
    t.push(def);
    Ok(())
}
