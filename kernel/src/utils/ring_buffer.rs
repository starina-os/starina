use core::cmp::min;
use core::mem::MaybeUninit;

pub struct RingBuffer<T, const SIZE: usize> {
    buffer: [MaybeUninit<T>; SIZE],
    head: usize,
    tail: usize,
}

impl<T: Copy, const SIZE: usize> RingBuffer<T, SIZE> {
    pub const fn new() -> Self {
        Self {
            buffer: [MaybeUninit::uninit(); SIZE],
            head: 0,
            tail: 0,
        }
    }

    pub fn write(&mut self, data: &[T]) {
        for &item in data {
            self.buffer[self.head % SIZE] = MaybeUninit::new(item);
            self.head = self.head.wrapping_add(1);
        }
    }

    pub fn read(&mut self, buf: &mut [T]) -> usize {
        let available = if self.head >= self.tail {
            self.head - self.tail
        } else {
            SIZE - self.tail + self.head
        };

        if available == 0 {
            return 0;
        }

        let len = min(buf.len(), available);
        for i in 0..len {
            unsafe {
                buf[i] = self.buffer[(self.tail + i) % SIZE].assume_init();
            }
        }

        self.tail = (self.tail + len) % SIZE;
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
        let read_len = buffer.read(&mut read_buf);

        assert_eq!(read_len, data.len());
        assert_eq!(&read_buf[..read_len], data);
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let mut buffer: RingBuffer<u8, 100> = RingBuffer::new();
        let large_data = vec![b'x'; 150];

        buffer.write(&large_data);

        let mut read_buf = [0u8; 100];
        let read_len = buffer.read(&mut read_buf);

        assert_eq!(read_len, 100);
        assert_eq!(read_buf, [b'x'; 100]);
    }

    #[test]
    fn test_ring_buffer_partial_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"0123456789";

        buffer.write(data);

        let mut small_buf = [0u8; 5];
        let read_len = buffer.read(&mut small_buf);

        assert_eq!(read_len, 5);
        assert_eq!(&small_buf, b"01234");
    }

    #[test]
    fn test_ring_buffer_multiple_reads() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello world";

        buffer.write(data);

        let mut first_buf = [0u8; 5];
        let first_len = buffer.read(&mut first_buf);
        assert_eq!(first_len, 5);
        assert_eq!(&first_buf, b"hello");

        let mut second_buf = [0u8; 6];
        let second_len = buffer.read(&mut second_buf);
        assert_eq!(second_len, 6);
        assert_eq!(&second_buf, b" world");
    }

    #[test]
    fn test_ring_buffer_empty_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();

        let mut read_buf = [0u8; 10];
        let read_len = buffer.read(&mut read_buf);

        assert_eq!(read_len, 0);
    }
}
