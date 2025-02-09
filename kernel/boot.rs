use crate::arch::halt;

pub fn boot() -> ! {
    println!("\nBooting Starina...");
    halt();
}
