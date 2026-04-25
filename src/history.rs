//! Fixed-capacity ring buffer used as the sample history for sparklines.

use std::array;

pub struct RingBuffer<T: Copy + Default, const N: usize> {
    buf: [T; N],
    /// Number of items written so far, capped at N.
    len: usize,
    /// Index where the next write will go.
    head: usize,
}

impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            buf: array::from_fn(|_| T::default()),
            len: 0,
            head: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        self.buf[self.head] = value;
        self.head = (self.head + 1) % N;
        if self.len < N {
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate items in chronological order (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        // Start index of the oldest element.
        let start = if self.len < N {
            0
        } else {
            self.head
        };
        (0..self.len).map(move |i| self.buf[(start + i) % N])
    }
}

impl<T: Copy + Default, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf: RingBuffer<f32, 4> = RingBuffer::new();

        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert_eq!(buf.capacity(), 4);
    }

    #[test]
    fn push_below_capacity_appends_in_order() {
        let mut buf: RingBuffer<f32, 4> = RingBuffer::new();

        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);

        let collected: Vec<f32> = buf.iter().collect();
        assert_eq!(collected, vec![1.0, 2.0, 3.0]);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn push_at_capacity_drops_oldest() {
        let mut buf: RingBuffer<f32, 3> = RingBuffer::new();

        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0);
        buf.push(5.0);

        let collected: Vec<f32> = buf.iter().collect();
        assert_eq!(collected, vec![3.0, 4.0, 5.0]);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn iter_handles_wraparound_correctly() {
        let mut buf: RingBuffer<u32, 4> = RingBuffer::new();

        for i in 1..=10u32 {
            buf.push(i);
        }

        let collected: Vec<u32> = buf.iter().collect();
        assert_eq!(collected, vec![7, 8, 9, 10]);
    }
}
