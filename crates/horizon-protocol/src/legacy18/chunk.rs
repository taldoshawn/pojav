//! Borrowed views of protocol 47 chunk columns. States are little-endian u16
//! values (block ID << 4 | metadata), not the modern global state registry.
//! All block arrays precede all block-light arrays, then sky-light and biomes.
use crate::{Error, Result, buf::Reader};

const STATES: usize = 8192;
const LIGHT: usize = 2048;

#[derive(Debug)]
pub struct Column<'a> {
    pub x: i32,
    pub z: i32,
    pub ground_up: bool,
    pub mask: u16,
    skylight: bool,
    unload: bool,
    data: &'a [u8],
}

#[derive(Debug)]
pub struct Section<'a> {
    pub y: u8,
    states: &'a [u8],
    block_light: &'a [u8],
    sky_light: Option<&'a [u8]>,
}

/// The bulk packet supplies skylight once, followed by all column headers and
/// then all column bodies. Views borrow the packet; blocks are never expanded
/// into temporary arrays just to inspect a section.
pub fn decode_bulk(payload: &[u8]) -> Result<Vec<Column<'_>>> {
    let mut r = Reader::new(payload);
    let skylight = r.bool()?;
    // Even an empty column carries a ten-byte header and 256 biome bytes.
    // Check that lower bound before allocating the header vector.
    let count = r.count(payload.len() / (10 + 256))?;
    let mut headers = Vec::with_capacity(count);
    let mut total = 0usize;
    for _ in 0..count {
        let (x, z, mask) = (r.i32()?, r.i32()?, r.u16()?);
        let len = expected_len(mask, true, skylight);
        total = total
            .checked_add(len)
            .ok_or_else(|| Error::new("chunk bulk length overflow"))?;
        headers.push((x, z, mask, len));
    }
    if r.remaining() != total {
        return Err(Error::new(
            "protocol 47 bulk chunk length does not match its headers",
        ));
    }
    headers
        .into_iter()
        .map(|(x, z, mask, len)| {
            Ok(Column {
                x,
                z,
                ground_up: true,
                mask,
                skylight,
                unload: false,
                data: r.bytes(len)?,
            })
        })
        .collect()
}

impl<'a> Column<'a> {
    /// Skylight comes from the current dimension (Overworld), not a guessed
    /// payload length. A respawn can change it before the next chunk arrives.
    pub fn decode(payload: &'a [u8], skylight: bool) -> Result<Self> {
        let mut r = Reader::new(payload);
        let x = r.i32()?;
        let z = r.i32()?;
        let ground_up = r.bool()?;
        let mask = r.u16()?;
        let expected = expected_len(mask, ground_up, skylight);
        let length = r.count(16 * (STATES + 2 * LIGHT) + 256)?;
        // An empty, ground-up column is also the legacy unload packet. Some
        // servers include the otherwise unused biome bytes; accept both forms.
        let unload = ground_up && mask == 0 && length == 0;
        if length != expected && !unload {
            return Err(Error::new(format!(
                "protocol 47 chunk length {length}, expected {expected}"
            )));
        }
        let data = r.bytes(length)?;
        super::finish(&r)?;
        Ok(Self {
            x,
            z,
            ground_up,
            mask,
            skylight,
            unload: ground_up && mask == 0,
            data,
        })
    }

    pub fn is_unload(&self) -> bool {
        self.unload
    }

    pub fn biomes(&self) -> Option<&'a [u8]> {
        if !self.ground_up || self.is_unload() {
            return None;
        }
        Some(&self.data[self.data.len() - 256..])
    }

    pub fn section(&self, y: u8) -> Option<Section<'a>> {
        if y >= 16 || self.mask & (1u16 << y) == 0 {
            return None;
        }
        let index = (self.mask & ((1u16 << y) - 1)).count_ones() as usize;
        let n = self.mask.count_ones() as usize;
        let block = n * STATES + index * LIGHT;
        let sky = n * (STATES + LIGHT) + index * LIGHT;
        Some(Section {
            y,
            states: &self.data[index * STATES..(index + 1) * STATES],
            block_light: &self.data[block..block + LIGHT],
            sky_light: self.skylight.then(|| &self.data[sky..sky + LIGHT]),
        })
    }
}

fn expected_len(mask: u16, ground_up: bool, skylight: bool) -> usize {
    mask.count_ones() as usize * (STATES + LIGHT + if skylight { LIGHT } else { 0 })
        + if ground_up { 256 } else { 0 }
}

impl Section<'_> {
    fn index(x: u8, y: u8, z: u8) -> Option<usize> {
        (x < 16 && y < 16 && z < 16)
            .then_some((usize::from(y) * 16 + usize::from(z)) * 16 + usize::from(x))
    }

    pub fn state(&self, x: u8, y: u8, z: u8) -> Option<u16> {
        let i = Self::index(x, y, z)? * 2;
        Some(u16::from_le_bytes([self.states[i], self.states[i + 1]]))
    }

    pub fn block_light(&self, x: u8, y: u8, z: u8) -> Option<u8> {
        Self::index(x, y, z).map(|i| nibble(self.block_light, i))
    }

    pub fn sky_light(&self, x: u8, y: u8, z: u8) -> Option<u8> {
        let i = Self::index(x, y, z)?;
        self.sky_light.map(|light| nibble(light, i))
    }
}

fn nibble(bytes: &[u8], i: usize) -> u8 {
    (bytes[i / 2] >> ((i & 1) * 4)) & 15
}
