use common::UObject;
use common::list::{self, List};
use crate::hooks::OUTLINE_COMPONENT;
use sdk::Engine::Pawn;
use sdk::FSD::OutlineComponent;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    List(#[from] list::Error),
}

struct PawnWrapper {
    pointer: *mut Pawn,
}

pub struct Pawns {
    pawns: List<PawnWrapper, 256>,
}

impl Pawns {
    pub const fn new() -> Self {
        Pawns {
            pawns: List::new(),
        }
    }

    pub unsafe fn add(&mut self, pawn: *mut Pawn) -> Result<(), Error> {
        self.pawns.push(PawnWrapper { pointer: pawn })?;
        Self::set_index(pawn, self.pawns.len() - 1);
        Self::set_outline(pawn);
        Ok(())
    }

    pub unsafe fn remove(&mut self, pawn: *mut Pawn) -> Result<(), Error> {
        // Find the index of the pawn to remove.
        let index = Self::index(pawn);

        // Remove the pawn by replacing its entry in the array with the last pawn.
        let removed = self.pawns.swap_remove(index)?;

        if removed.pointer == pawn {
            // If we replaced with a different pawn,
            if index < self.pawns.len() {
                // Then update that pawn's internal index to reflect its new position in the array.
                let p = self.pawns.get_mut(index)?;
                Self::set_index(p.pointer, index);
            }
        } else {
            // The pawn we removed wasn't the pawn that was passed in.
            // This can happen because the passed-in pawn is untracked but shares the same index as a tracked pawn.
            // Re-add the tracked pawn.
            self.add(removed.pointer)?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    unsafe fn print(&self) {
        if self.pawns.is_empty() {
            common::log!("Tracking no pawns.");
        } else {
            for (i, pawn) in self.pawns.iter().enumerate() {
                common::log!("[{}] {} {}", i, common::Hex(pawn.pointer), (*pawn.pointer.cast::<common::UObject>()).name());
            }
        }
    }

    unsafe fn set_index(pawn: *mut Pawn, index: usize) {
        (*pawn).pad_at_0x168[1] = index as u8;
    }

    unsafe fn index(pawn: *mut Pawn) -> usize {
        usize::from((*pawn).pad_at_0x168[1])
    }

    pub fn clear(&mut self) {
        self.pawns.clear();
    }

    unsafe fn set_outline(pawn: *mut Pawn) {
        for &component in (*pawn).BlueprintCreatedComponents.iter() {
            if (*component.cast::<UObject>()).is(OUTLINE_COMPONENT) {
                let component = component.cast::<OutlineComponent>();
                (*component).UnlockOutline();
                (*component).ToggleDefaultOutline(true);
                (*component).LockOutline();
            }
        }
    }
}