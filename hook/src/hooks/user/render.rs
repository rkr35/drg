#[derive(macros::NoPanicErrorDebug)]
enum Error {
    UnknownViewModeIndex(i32),
}


#[repr(i32)]
enum ViewModeIndex {
    Unlit = 2,
    Lit = 3,
}

impl TryFrom<i32> for ViewModeIndex {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self::Unlit),
            3 => Ok(Self::Lit),
            _ => Err(Error::UnknownViewModeIndex(value)),
        }
    }
}

unsafe fn view_mode_ptr() -> *mut i32 {
    const OFFSET_VIEW_MODE_INDEX: usize = 0xB0;
    (*crate::GEngine).GameViewport.cast::<u8>().add(OFFSET_VIEW_MODE_INDEX).cast::<i32>()
}

unsafe fn set_view_mode_index(mode: ViewModeIndex) {
    *view_mode_ptr() = mode as i32;
}

unsafe fn get_view_mode_index() -> Result<ViewModeIndex, Error> {
    ViewModeIndex::try_from(*view_mode_ptr())
}

pub unsafe fn remove_lighting() {
    set_view_mode_index(ViewModeIndex::Unlit);
}

pub unsafe fn restore_lighting() {
    set_view_mode_index(ViewModeIndex::Lit);
}

pub unsafe fn toggle_lighting() {
    match get_view_mode_index() {
        Ok(ViewModeIndex::Unlit) => restore_lighting(),
        Ok(ViewModeIndex::Lit) => remove_lighting(),
        Err(e) => common::log!("toggle_lighting() error: {:?}", e),
    }
}