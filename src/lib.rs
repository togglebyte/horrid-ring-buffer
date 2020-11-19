use std::alloc::{dealloc, alloc, Layout};
use std::io::{Read, Write, Result};
use std::mem::{size_of, align_of};
use std::ptr;

// -----------------------------------------------------------------------------
//     - Ring buffer -
// -----------------------------------------------------------------------------
pub struct HorridRing<T> {
    read: usize,
    write: usize,
    write_wrap: u8,
    read_wrap: u8,
    inner: *mut T,
    capacity: usize,
}

impl<T> HorridRing<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity * size_of::<T>(), align_of::<T>())
            .expect("could not layout");

        let mem = unsafe { alloc(layout) };
        let inner = mem.cast::<T>();

        Self {
            read: 0,
            write: 0,
            write_wrap: 0,
            read_wrap: 0,
            inner,
            capacity,
        }
    }

    pub fn push(&mut self, val: T) {
        unsafe {
            ptr::write(self.inner.offset(self.write as isize), val);
            self.write = (self.write + 1) % self.capacity;
            if self.write == 0 {
                self.write_wrap = self.write_wrap.wrapping_add(1);
            }
        }
    }

    pub fn clear(&mut self) {
        self.write = 0;
        self.read = 0;
        self.write_wrap = 0;
        self.read_wrap = 0;
    }

    pub fn drain(&mut self) -> Vec<T> {
        let ret_val = self.collect::<Vec<_>>();
        self.clear();
        ret_val
    }
}

// -----------------------------------------------------------------------------
//     - Iterator impl -
// -----------------------------------------------------------------------------
impl<T> Iterator for HorridRing<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read_wrap == self.write_wrap && self.read == self.write {
            return None;
        }

        let p = if self.read_wrap < self.write_wrap && self.read < self.write {
            self.read = self.write;
            let p = unsafe { self.inner.offset(self.read as isize).read() };
            p
        } else {
            let p = unsafe { self.inner.offset(self.read as isize).read() };
            p
        };

        self.read = (self.read + 1) % self.capacity;
        if self.read == 0 {
            self.read_wrap = self.read_wrap.wrapping_add(1);
        }

        Some(p)
    }
}

// -----------------------------------------------------------------------------
//     - Read impl -
// -----------------------------------------------------------------------------
impl Read for HorridRing<u8> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut index = 0;
        let buf_len = buf.len();
        while let Some(val) = self.next() {
            buf[index] = val;
            index += 1;
            if index == buf_len {
                break;
            }
        }

        Ok(index)
    }
}

// -----------------------------------------------------------------------------
//     - Write impl -
// -----------------------------------------------------------------------------
impl Write for HorridRing<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        buf.iter().for_each(|b| self.push(*b));
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
//     - Drop impl -
// -----------------------------------------------------------------------------

impl<T> Drop for HorridRing<T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(self.capacity * size_of::<T>(), align_of::<T>()) .expect("could not layout");
            dealloc(self.inner.cast::<u8>(), layout);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_read_empty() {
        let mut rb = HorridRing::<u8>::with_capacity(4);
        assert!(rb.next().is_none());
    }

    #[test]
    fn test_write_wraps() {
        let mut rb = HorridRing::with_capacity(2);
        rb.push(0); // 0 [0] [?]
        rb.push(1); // 1 [0] [1]
        rb.push(2); // 2 [2] [1]
        assert_eq!(rb.next(), Some(1));
        assert_eq!(rb.next(), Some(2));
    }

    #[test]
    fn test_non_wrapping_write() {
        let mut rb = HorridRing::with_capacity(2);
        rb.push(0);
        rb.push(1);
        assert_eq!(rb.next(), Some(0));
        assert_eq!(rb.next(), Some(1));
    }

    #[test]
    fn test_read() {
        let mut buf = [0;1024];
        let mut rb = HorridRing::with_capacity(4);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4);

        let num_bytes = rb.read(&mut buf).unwrap();
        assert_eq!(&buf[0..num_bytes], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_write() {
        let buf = [3;1024];
        let mut rb = HorridRing::with_capacity(2);

        let bytes_written = rb.write(&buf).unwrap();
        assert_eq!(bytes_written, buf.len());
        assert_eq!(rb.drain(), vec![3, 3]);
    }

    #[test]
    fn test_clear() {
        let mut rb = HorridRing::with_capacity(4);
        rb.push(1);
        rb.clear();

        assert!(rb.next().is_none());
    }

    #[test]
    fn test_drain() {
        let mut rb = HorridRing::with_capacity(4);
        rb.push(1);
        rb.push(2);
        let val = rb.drain();

        assert_eq!(val, vec![1, 2]);
    }
}
