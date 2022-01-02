use sdk::Engine::GameViewportClient;

#[repr(i32)]
enum ViewModeIndex {
    Unlit = 2,
    Lit = 3,
}

pub unsafe fn remove_lighting() {
    set_view_mode_index((*crate::GEngine).GameViewport, ViewModeIndex::Unlit);
}

pub unsafe fn restore_lighting() {
    set_view_mode_index((*crate::GEngine).GameViewport, ViewModeIndex::Lit);
}

unsafe fn set_view_mode_index(viewport: *mut GameViewportClient, mode: ViewModeIndex) {
    const VIEW_MODE_INDEX: usize = 0xA8;
    *viewport.cast::<u8>().add(VIEW_MODE_INDEX).cast::<i32>() = mode as i32;
}