use crate::arch::halt;

pub struct App2;
impl starina::Worker for App2 {
    type Context = usize;
    fn init() -> Self {
        App2
    }
}
pub fn boot() -> ! {
    {
        use alloc::boxed::Box;

        use starina::Worker;

        let ktest = ktest::App::init();
        let app2 = App2::init();
        // let apps: [Box<dyn starina::Worker>; 2] = [Box::new(ktest), Box::new(app2)];
    }

    println!("\nBooting Starina...");
    halt();
}
