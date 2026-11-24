//! Parser for the classic client's `.imf` player-layer order files
//! (`data\imf\<job>_<sex>.imf`).
//!
//! An `.imf` stores a draw priority per action and frame for the body and the
//! head layers. The frame with the higher priority is drawn first, which is how
//! the official client hides the head behind the body during some actions.
//!
//! The data file path is exposed by `world::jobs::imf_path`; this module only
//! parses the bytes. Wiring the resulting order into the animation renderer is
//! the remaining renderer hook (the current entity composition always draws
//! body, then head/headgear), so the parsed table is kept as a tested API for
//! that follow-up instead of being half-wired.

use std::io;

/// Per-action/frame flag: `true` when the head is drawn before (behind) the
/// body.
pub(crate) struct Imf {
    /// Indexed by action, then frame; missing actions/frames are `false` (body
    /// first).
    head_behind_body: Vec<Vec<bool>>,
}

impl Imf {
    /// Parses an `.imf` payload.
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut reader = Reader { bytes, position: 0 };
        let _version = reader.f32()?;
        let _checksum = reader.i32()?;
        let last_layer = reader.i32()?;
        let mut layers = Vec::new();

        for _ in 0..=last_layer.clamp(0, 1) {
            let action_count = reader.i32()?.max(0) as usize;
            let mut actions = Vec::with_capacity(action_count);
            for _ in 0..action_count {
                let frame_count = reader.i32()?.max(0) as usize;
                let mut frames = Vec::with_capacity(frame_count);
                for _ in 0..frame_count {
                    frames.push(reader.i32()?);
                    // The position point is not needed for the layer order.
                    reader.take(8)?;
                }
                actions.push(frames);
            }
            layers.push(actions);
        }

        let [body, head] =
            <[Vec<Vec<i32>>; 2]>::try_from(layers).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "IMF without a head layer"))?;

        let head_behind_body = body
            .iter()
            .zip(&head)
            .map(|(body, head)| body.iter().zip(head).map(|(body, head)| head > body).collect())
            .collect();

        Ok(Self { head_behind_body })
    }

    /// Whether the head of `action`/`frame` is drawn behind the body.
    pub fn head_behind_body(&self, action: usize, frame: usize) -> bool {
        self.head_behind_body
            .get(action)
            .and_then(|frames| frames.get(frame))
            .copied()
            .unwrap_or(false)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take(&mut self, count: usize) -> io::Result<&[u8]> {
        let end = self.position.saturating_add(count);
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "truncated IMF file"))?;
        self.position = end;
        Ok(bytes)
    }

    fn i32(&mut self) -> io::Result<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes(bytes.try_into().expect("four bytes")))
    }

    fn f32(&mut self) -> io::Result<f32> {
        let bytes = self.take(4)?;
        Ok(f32::from_le_bytes(bytes.try_into().expect("four bytes")))
    }
}

#[cfg(test)]
mod tests {
    use super::Imf;

    fn sample() -> Vec<u8> {
        let mut bytes = [1.01f32.to_le_bytes(), 0i32.to_le_bytes(), 1i32.to_le_bytes()].concat();
        let layers: [[&[i32]; 2]; 2] = [[&[1], &[1, 0]], [&[0], &[0, 1]]];
        for priorities in layers {
            bytes.extend(2i32.to_le_bytes());
            for frames in priorities {
                bytes.extend((frames.len() as i32).to_le_bytes());
                for &priority in frames {
                    bytes.extend([priority, 0, 0].iter().flat_map(|value| value.to_le_bytes()));
                }
            }
        }
        bytes
    }

    #[test]
    fn parses_head_behind_body_per_action_and_frame() {
        let imf = Imf::parse(&sample()).expect("valid IMF");
        assert!(!imf.head_behind_body(0, 0));
        assert!(!imf.head_behind_body(1, 0));
        assert!(imf.head_behind_body(1, 1));
        assert!(!imf.head_behind_body(99, 99));
    }

    #[test]
    fn rejects_truncated_files() {
        let bytes = sample();
        assert!(Imf::parse(&bytes[..bytes.len() - 4]).is_err());
    }
}
