//! Where an item keeps its mesh (stage 2 of the packed-mesh plan).
//!
//! A `MeshSlot` holds a mesh packed (`PackedMesh`), open (today's `Mesh`
//! behind a `Shared` handle), or both: an open copy that matches its packed
//! form exactly.
//!
//! - Reading through `Deref` (`item.mesh.faces`, `&*item.mesh`) gives the
//!   open `Mesh`, opening a packed slot first. That open is allowed but
//!   counted (`read_opens`), so tests can catch code that quietly opens
//!   meshes nobody is editing. Code that only looks at a mesh should use
//!   `read()`, which never opens anything.
//! - Writing through `DerefMut` (`item.mesh.vertices[i] = p`) opens the slot,
//!   drops the packed form (it no longer matches) and takes a new content
//!   stamp. Sharing works as with `Shared<Mesh>`: a slot shared with a
//!   history checkpoint is copied before its first change.
//! - The content stamp changes only when the contents may have changed:
//!   on `DerefMut`, on assigning a new mesh, and never on packing or
//!   opening. Caches can key on it instead of on a pointer.
//!
//! Nothing packs a slot on its own yet; the idle packer is stage 3. Until
//! then a slot packs only when `pack()` is called, so the app behaves exactly
//! as before.
use crate::packed_mesh::{MeshRead, PackedMesh, worth_packing};
use crate::{Mesh, Result, Shared};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAMP: AtomicU64 = AtomicU64::new(1);
static READ_OPENS: AtomicU64 = AtomicU64::new(0);

fn next_stamp() -> u64 {
    NEXT_STAMP.fetch_add(1, Ordering::Relaxed)
}

/// How many times a packed slot has been opened only to be read, since the
/// program started. Tests compare it before and after the code they check.
pub fn read_opens() -> u64 {
    READ_OPENS.load(Ordering::Relaxed)
}

pub struct MeshSlot {
    /// The open form. Always set unless the slot is packed.
    open: OnceLock<Shared<Mesh>>,
    /// The packed form. When both are set they hold the same contents.
    packed: Option<PackedMesh>,
    stamp: u64,
}

/// A read-only view of a slot's mesh in whichever form it has, without
/// opening it.
#[derive(Clone, Copy)]
pub enum MeshView<'a> {
    Open(&'a Mesh),
    Packed(&'a PackedMesh),
}
impl MeshView<'_> {
    /// The view as a `MeshRead`, for code written against the trait.
    pub fn as_read(&self) -> &dyn MeshRead {
        match self {
            Self::Open(m) => *m,
            Self::Packed(p) => *p,
        }
    }
}

impl MeshSlot {
    pub fn new(mesh: impl Into<Shared<Mesh>>) -> Self {
        Self {
            open: OnceLock::from(mesh.into()),
            packed: None,
            stamp: next_stamp(),
        }
    }

    /// A slot holding only `packed`.
    pub fn from_packed(packed: PackedMesh) -> Self {
        Self {
            open: OnceLock::new(),
            packed: Some(packed),
            stamp: next_stamp(),
        }
    }

    /// The content stamp: equal stamps mean equal contents. Packing and
    /// opening keep it; any change gives a new one.
    pub fn stamp(&self) -> u64 {
        self.stamp
    }

    /// True when the slot has no open copy (reading it through `Deref`
    /// would open it).
    pub fn is_packed(&self) -> bool {
        self.open.get().is_none()
    }

    /// True when the slot holds a packed form (with or without an open copy).
    pub fn has_packed(&self) -> bool {
        self.packed.is_some()
    }

    /// The mesh in whichever form the slot holds, never opening it. The open
    /// form wins when both are present, since it is what tools edit.
    pub fn read(&self) -> MeshView<'_> {
        match (self.open.get(), &self.packed) {
            (Some(open), _) => MeshView::Open(open),
            (None, Some(packed)) => MeshView::Packed(packed),
            (None, None) => unreachable!("a mesh slot always holds one form"),
        }
    }

    /// The open mesh's shared handle, opening a packed slot (counted).
    pub fn shared(&self) -> &Shared<Mesh> {
        self.open.get_or_init(|| {
            READ_OPENS.fetch_add(1, Ordering::Relaxed);
            let packed = self
                .packed
                .as_ref()
                .expect("a mesh slot always holds one form");
            Shared::new(packed.to_mesh())
        })
    }

    /// Opens the slot for editing: drops the packed form and takes a new
    /// stamp. Returns the shared handle so callers can replace it whole.
    pub fn shared_mut(&mut self) -> &mut Shared<Mesh> {
        if self.open.get().is_none() {
            let packed = self
                .packed
                .take()
                .expect("a mesh slot always holds one form");
            // Opening for an edit is expected, so it isn't counted.
            let _ = self.open.set(Shared::new(packed.to_mesh()));
        }
        self.packed = None;
        self.stamp = next_stamp();
        self.open.get_mut().expect("opened above")
    }

    /// Packs the slot and drops its open copy. Small meshes (under
    /// `PACK_MIN_POLYGONS`) and meshes that can't be packed stay open.
    /// Returns whether the slot is packed afterwards. Keeps the stamp.
    pub fn pack(&mut self) -> bool {
        if self.packed.is_none() {
            let Some(open) = self.open.get() else {
                return true;
            };
            if !worth_packing(open) {
                return false;
            }
            match PackedMesh::from_mesh(open) {
                Ok(packed) => self.packed = Some(packed),
                Err(_) => return false,
            }
        }
        self.open = OnceLock::new();
        true
    }

    /// Opens the slot for reading without counting it as a hidden open
    /// (for the active mesh, which is opened on purpose). Keeps the packed
    /// form and the stamp.
    pub fn open(&self) -> &Mesh {
        self.open.get_or_init(|| {
            Shared::new(
                self.packed
                    .as_ref()
                    .expect("a mesh slot always holds one form")
                    .to_mesh(),
            )
        })
    }

    /// True when both slots hold the same contents without comparing them:
    /// the same stamp, or the same shared open allocation. False does not
    /// mean different.
    pub fn same(a: &Self, b: &Self) -> bool {
        a.stamp == b.stamp
            || matches!((a.open.get(), b.open.get()), (Some(x), Some(y)) if Shared::same(x, y))
    }

    /// An owned copy of the open mesh.
    pub fn to_owned_value(&self) -> Mesh {
        self.shared().to_owned_value()
    }

    /// `Mesh::validate`, memoised on the open allocation (see `shared.rs`).
    pub fn validate(&self) -> Result<()> {
        self.shared().validate()
    }
}

impl Default for MeshSlot {
    fn default() -> Self {
        Self::new(Mesh::default())
    }
}
impl Clone for MeshSlot {
    fn clone(&self) -> Self {
        let open = OnceLock::new();
        if let Some(mesh) = self.open.get() {
            let _ = open.set(mesh.clone());
        }
        Self {
            open,
            packed: self.packed.clone(),
            stamp: self.stamp,
        }
    }
}
impl std::ops::Deref for MeshSlot {
    type Target = Mesh;
    fn deref(&self) -> &Mesh {
        self.shared()
    }
}
impl std::ops::DerefMut for MeshSlot {
    fn deref_mut(&mut self) -> &mut Mesh {
        self.shared_mut()
    }
}
impl From<Mesh> for MeshSlot {
    fn from(mesh: Mesh) -> Self {
        Self::new(mesh)
    }
}
impl From<Shared<Mesh>> for MeshSlot {
    fn from(mesh: Shared<Mesh>) -> Self {
        Self::new(mesh)
    }
}
impl From<&MeshSlot> for Shared<Mesh> {
    fn from(slot: &MeshSlot) -> Self {
        slot.shared().clone()
    }
}
impl From<MeshSlot> for Shared<Mesh> {
    fn from(slot: MeshSlot) -> Self {
        slot.shared().clone()
    }
}
impl PartialEq for MeshSlot {
    fn eq(&self, other: &Self) -> bool {
        Self::same(self, other) || **self == **other
    }
}
/// For caches that still hold a `Shared<Mesh>` key: equal when the slot's
/// open copy is that allocation, or the contents match.
impl PartialEq<MeshSlot> for Shared<Mesh> {
    fn eq(&self, slot: &MeshSlot) -> bool {
        let open = slot.shared();
        Shared::same(self, open) || **self == **open
    }
}
impl PartialEq<Mesh> for MeshSlot {
    fn eq(&self, other: &Mesh) -> bool {
        **self == *other
    }
}
impl std::fmt::Debug for MeshSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.read() {
            MeshView::Open(mesh) => mesh.fmt(f),
            MeshView::Packed(packed) => write!(
                f,
                "PackedMesh({} points, {} polygons)",
                packed.point_count(),
                packed.polygon_count()
            ),
        }
    }
}
impl Serialize for MeshSlot {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self.read() {
            MeshView::Open(mesh) => mesh.serialize(s),
            // Stage 4 writes packed arrays directly; until then a packed
            // slot is saved through a temporary open copy.
            MeshView::Packed(packed) => packed.to_mesh().serialize(s),
        }
    }
}
impl<'de> Deserialize<'de> for MeshSlot {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Mesh::deserialize(d).map(Self::new)
    }
}
