use starina::error::ErrorCode;

pub struct VCpu {}

impl VCpu {
    pub fn new() -> Result<VCpu, ErrorCode> {
        Ok(VCpu {})
    }
}

pub fn vcpu_entry() -> ! {
    todo!()
}
