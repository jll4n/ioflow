pub mod diff;
pub mod svg;

use crate::types::LdElement;

/// Couleur de mise en évidence d'un élément dans le rendu SVG.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ElemColor {
    Normal,
    Added,
    Removed,
    Modified,
}

impl ElemColor {
    pub fn stroke(self) -> &'static str {
        match self {
            Self::Normal => "#1a1a1a",
            Self::Added => "#16a34a",
            Self::Removed => "#dc2626",
            Self::Modified => "#d97706",
        }
    }

    pub fn fill(self) -> &'static str {
        match self {
            Self::Normal => "white",
            Self::Added => "#dcfce7",
            Self::Removed => "#fee2e2",
            Self::Modified => "#fef9c3",
        }
    }
}

pub(crate) fn element_local_id(elem: &LdElement) -> u32 {
    match elem {
        LdElement::LeftPowerRail(r) => r.local_id,
        LdElement::RightPowerRail(r) => r.local_id,
        LdElement::Contact(c) => c.local_id,
        LdElement::Coil(c) => c.local_id,
        LdElement::Block(b) => b.local_id,
    }
}
