# Writing Your First Device Driver

## Scaffolding

## Look for a device interface in IDL

## Goldfish RTC device 101

[specification](https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-2.0-release/docs/GOLDFISH-VIRTUAL-HARDWARE.TXT#:~:text=Goldfish%20real%2Dtime%20clock).

## Detecting the device with Device Tree

## Mapping MMIO registers

## Avoid boring stuff with awesome public crates

## Allocating DMA memory

Use `MappedFolio::create` to allocate a physically-contiguous memory block, and  `starina_driver_utils::buffer_pool::BufferPool` to manage the allocated DMA space.

## Handling hardware interrupts

## Next Steps
