use starina::channel::Channel;

use crate::App;

// TODO: Remove this.
pub fn app_main(handle_id: isize) {
    use starina::handle::HandleId;
    use starina::handle::OwnedHandle;

    let handle_id = HandleId::from_raw(handle_id.try_into().unwrap());
    let handle = OwnedHandle::from_raw(handle_id);
    let ch = Channel::from_handle(handle);
    starina::eventloop::app_loop::<App>(ch);
}
