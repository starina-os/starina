use core::cmp::min;
use core::mem::MaybeUninit;

pub struct RingBuffer<T, const SIZE: usize> {
    buffer: [MaybeUninit<T>; SIZE],
    head: usize,
}

impl<T: Copy, const SIZE: usize> RingBuffer<T, SIZE> {
    pub const fn new() -> Self {
        Self {
            buffer: [MaybeUninit::uninit(); SIZE],
            head: 0,
        }
    }

    pub fn write(&mut self, data: &[T]) {
        for &item in data {
            self.buffer[self.head % SIZE] = MaybeUninit::new(item);
            self.head = self.head.wrapping_add(1);
        }
    }

    pub fn read(&mut self, offset: usize, buf: &mut [T]) -> usize {
        if offset > self.head {
            return 0;
        }

        let avail = self.head.wrapping_sub(offset);
        if avail > SIZE {
            return 0;
        }

        let len = min(buf.len(), avail);
        for i in 0..len {
            unsafe {
                buf[i] = self.buffer[(offset + i) % SIZE].assume_init();
            }
        }

        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_write_and_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello world";

        buffer.write(data);

        let mut read_buf = [0u8; 20];
        let read_len = buffer.read(0, &mut read_buf);

        assert_eq!(read_len, data.len());
        assert_eq!(&read_buf[..read_len], data);
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let mut buffer: RingBuffer<u8, 100> = RingBuffer::new();
        let large_data = vec![b'x'; 150];

        buffer.write(&large_data);

        let mut read_buf = [0u8; 100];
        let read_len = buffer.read(0, &mut read_buf);

        assert_eq!(read_len, 0);
    }

    #[test]
    fn test_ring_buffer_partial_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"0123456789";

        buffer.write(data);

        let mut small_buf = [0u8; 5];
        let read_len = buffer.read(0, &mut small_buf);

        assert_eq!(read_len, 5);
        assert_eq!(&small_buf, b"01234");
    }

    #[test]
    fn test_ring_buffer_offset_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello world";

        buffer.write(data);

        let mut read_buf = [0u8; 5];
        let read_len = buffer.read(6, &mut read_buf);

        assert_eq!(read_len, 5);
        assert_eq!(&read_buf, b"world");
    }

    #[test]
    fn test_ring_buffer_out_of_bounds_offset() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello";

        buffer.write(data);

        let mut read_buf = [0u8; 10];
        let read_len = buffer.read(10, &mut read_buf);

        assert_eq!(read_len, 0);
    }
}
