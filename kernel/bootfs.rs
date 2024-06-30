use core::mem::size_of;
use core::slice;
use core::str::from_utf8_unchecked;

use ftl_types::bootfs::BootfsEntry;
use ftl_types::bootfs::BootfsHeader;
use ftl_types::bootfs::BOOTFS_MAGIC;

/// A workaround for the lack of alignment attribute on `include_bytes!`.
#[repr(align(4096))]
struct PageAligned<T: ?Sized>(T);

const BOOTFS_IMAGE: &PageAligned<[u8]> = &PageAligned(*include_bytes!("../build/bootfs.bin"));

/// Converts a null-terminated C string to `&str`.
///
/// # Panics
///
/// Panics if the input is not null-terminated.
///
/// # Safety
///
/// This function assumes that the input is a valid UTF-8 string.
pub unsafe fn cstr2str(cstr: &[u8]) -> &str {
    let len = cstr.iter().position(|&c| c == b'\0').unwrap();
    unsafe { from_utf8_unchecked(&cstr[..len]) }
}

pub struct FilesIter {
    image: *const u8,
    iter: slice::Iter<'static, BootfsEntry>,
}

impl Iterator for FilesIter {
    type Item = File;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.iter.next()?;
        // SAFETY: We assume the mkbootfs tool correctly generated the image.
        let name = unsafe { cstr2str(&entry.name) };

        // SAFETY: We assume the mkbootfs tool correctly generated the image.
        let data = unsafe {
            slice::from_raw_parts(self.image.add(entry.offset as usize), entry.size as usize)
        };

        Some(File { data, name })
    }
}

pub struct File {
    pub name: &'static str,
    pub data: &'static [u8],
}

pub struct Bootfs {
    image: *const u8,
    entries: &'static [BootfsEntry],
}

impl Bootfs {
    pub fn load() -> Bootfs {
        let image = BOOTFS_IMAGE.0.as_ptr();

        // SAFETY: PageAligned guarantees that the data is correctly aligned.
        let header = unsafe { &*(image as *const BootfsHeader) };
        assert_eq!(header.magic, BOOTFS_MAGIC);

        let entries = unsafe {
            core::slice::from_raw_parts(
                image.add(size_of::<BootfsHeader>()) as *const BootfsEntry,
                header.num_entries as usize,
            )
        };

        Bootfs {
            image: BOOTFS_IMAGE.0.as_ptr(),
            entries,
        }
    }

    pub fn files(&self) -> FilesIter {
        FilesIter {
            image: self.image,
            iter: self.entries.iter(),
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<File> {
        for f in self.files() {
            if f.name == name {
                return Some(f);
            }
        }

        None
    }
}
