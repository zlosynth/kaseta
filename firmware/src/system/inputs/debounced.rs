#[derive(Debug, Eq, PartialEq, defmt::Format)]
pub struct Debounced<const N: usize> {
    debounce_filter: DebounceBuffer<N>,
    active: bool,
}

impl<const N: usize> Debounced<N> {
    pub fn new() -> Self {
        Self {
            debounce_filter: DebounceBuffer::new(),
            active: false,
        }
    }

    pub fn update(&mut self, value: bool) -> bool {
        self.debounce_filter.write(value);
        self.active = self.debounce_filter.read();
        self.active
    }
}

#[derive(Debug, Eq, PartialEq, defmt::Format)]
pub struct DebounceBuffer<const N: usize> {
    buffer: [bool; N],
    pointer: usize,
}

impl<const N: usize> DebounceBuffer<N> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            buffer: [false; N],
            pointer: 0,
        }
    }

    pub fn write(&mut self, value: bool) {
        self.buffer[self.pointer] = value;
        self.pointer = (self.pointer + 1) % N;
    }

    pub fn read(&self) -> bool {
        let up: usize = self.buffer.iter().filter(|i| **i).count();
        up > N / 2
    }
}
