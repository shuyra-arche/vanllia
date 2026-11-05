/// A separate versioned measurement frame. Native lifecycle frames decide success.
pub const ITERATIONS: u64 = 100_000;
const PREFIX: &[u8; 6] = b"\x1dVNM1:";

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub iterations: u64,
    pub elapsed_ns: u64,
    pub checksum: u64,
    pub task: u64,
}

impl Sample {
    #[cfg(target_os = "none")]
    pub fn encode(self) -> [u8; 71] {
        let mut frame = [b'\n'; 71];
        frame[..6].copy_from_slice(PREFIX);
        let words = [self.iterations, self.elapsed_ns, self.checksum, self.task];
        for (at, byte) in words.iter().flat_map(|n| n.to_le_bytes()).enumerate() {
            frame[6 + at * 2] = b"0123456789abcdef"[(byte >> 4) as usize];
            frame[7 + at * 2] = b"0123456789abcdef"[(byte & 15) as usize];
        }
        frame
    }

    #[cfg(not(target_os = "none"))]
    pub fn decode(serial: &[u8]) -> Result<Self, String> {
        let frames: Vec<_> = serial
            .windows(PREFIX.len())
            .enumerate()
            .filter(|(_, bytes)| *bytes == PREFIX)
            .map(|(at, _)| at)
            .collect();
        let [start] = frames.as_slice() else {
            return Err("expected exactly one measurement frame".into());
        };
        let rest = &serial[start + PREFIX.len()..];
        let end = rest
            .iter()
            .position(|b| *b == b'\n')
            .ok_or("truncated measurement")?;
        let raw = rest[..end].strip_suffix(b"\r").unwrap_or(&rest[..end]);
        if raw.len() != 64 {
            return Err("measurement length".into());
        }
        let mut words = [0u64; 4];
        for (word, digits) in words.iter_mut().zip(raw.as_chunks::<16>().0) {
            let mut bytes = [0; 8];
            for (byte, pair) in bytes.iter_mut().zip(digits.as_chunks::<2>().0) {
                let nibble = |n| match n {
                    b'0'..=b'9' => Ok(n - b'0'),
                    b'a'..=b'f' => Ok(n - b'a' + 10),
                    _ => Err("invalid measurement hex"),
                };
                *byte = nibble(pair[0])? * 16 + nibble(pair[1])?;
            }
            *word = u64::from_le_bytes(bytes);
        }
        let [iterations, elapsed_ns, checksum, task] = words;
        if iterations != ITERATIONS
            || elapsed_ns == 0
            || task == 0
            || checksum != task.wrapping_mul(iterations)
        {
            return Err("measurement identity/checksum".into());
        }
        Ok(Self {
            iterations,
            elapsed_ns,
            checksum,
            task,
        })
    }
}
