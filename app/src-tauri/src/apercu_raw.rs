//! Preview of RAW camera files: nearly all of them (Sony ARW, Canon CR2
//! and CR3, Nikon NEF, Fuji RAF, Olympus ORF, Panasonic RW2, DNG...) carry
//! a JPEG computed by the camera. The largest one is extracted, by walking
//! JPEG structures (not by guessing), with the orientation read from the
//! RAW's TIFF header (the embedded preview is not rotated).

use std::ops::Range;

/// End of the JPEG starting at `debut` (just after its EOI), by walking its
/// segments, or `None` if it is not a complete JPEG.
fn fin_jpeg(d: &[u8], debut: usize) -> Option<usize> {
    let mut i = debut + 2;
    loop {
        if i + 4 > d.len() || d[i] != 0xFF {
            return None;
        }
        let marqueur = d[i + 1];
        match marqueur {
            0xD9 => return Some(i + 2),
            0x01 | 0xD0..=0xD7 | 0xFF => i += if marqueur == 0xFF { 1 } else { 2 },
            0xDA => {
                // Start of scan: entropy-coded data up to the next marker that
                // is neither a stuffed byte nor a restart.
                let longueur = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
                let mut k = i + 2 + longueur;
                while k + 1 < d.len() {
                    if d[k] == 0xFF && d[k + 1] != 0x00 && !(0xD0..=0xD7).contains(&d[k + 1]) {
                        break;
                    }
                    k += 1;
                }
                i = k;
            }
            _ => i += 2 + u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize,
        }
    }
}

/// Width and height of a JPEG (its first frame header).
pub fn dimensions_jpeg(j: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 9 < j.len() {
        if j[i] != 0xFF {
            return None;
        }
        let m = j[i + 1];
        if matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let h = u16::from_be_bytes([j[i + 5], j[i + 6]]) as u32;
            let w = u16::from_be_bytes([j[i + 7], j[i + 8]]) as u32;
            return Some((w, h));
        }
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) {
            i += 2;
        } else {
            i += 2 + u16::from_be_bytes([j[i + 2], j[i + 3]]) as usize;
        }
    }
    None
}

/// The largest complete JPEG inside `d`.
pub fn plus_grand_jpeg(d: &[u8]) -> Option<Range<usize>> {
    let mut meilleur: Option<Range<usize>> = None;
    let mut i = 0;
    while let Some(p) = d[i..].windows(3).position(|w| w == [0xFF, 0xD8, 0xFF]) {
        let debut = i + p;
        if let Some(fin) = fin_jpeg(d, debut)
            && dimensions_jpeg(&d[debut..fin]).is_some()
            && meilleur.as_ref().is_none_or(|m| fin - debut > m.len())
        {
            meilleur = Some(debut..fin);
        }
        i = debut + 3;
    }
    meilleur
}

/// EXIF orientation (1 to 8) from the TIFF header of TIFF-based RAW files
/// (ARW, CR2, NEF, DNG, ORF, RW2, PEF...).
pub fn orientation_tiff(d: &[u8]) -> Option<u16> {
    let petit = match d.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_ = |o: usize| d.get(o..o + 2).map(|b| if petit { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) });
    let u32_ = |o: usize| {
        d.get(o..o + 4).map(|b| if petit { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) } else { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) })
    };
    let ifd = u32_(4)? as usize;
    let n = u16_(ifd)? as usize;
    (0..n.min(512)).find_map(|k| {
        let e = ifd + 2 + k * 12;
        (u16_(e)? == 0x0112).then(|| u16_(e + 8)).flatten().filter(|o| (1..=8).contains(o))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny JPEG-like structure: SOI, SOF0 (w x h), SOS with data, EOI.
    fn jpeg(w: u16, h: u16, donnees: usize) -> Vec<u8> {
        let mut j = vec![0xFF, 0xD8];
        j.extend([0xFF, 0xC0, 0x00, 0x0B, 0x08]);
        j.extend(h.to_be_bytes());
        j.extend(w.to_be_bytes());
        j.extend([0x01, 0x01, 0x11, 0x00]);
        j.extend([0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
        // Entropy data with a stuffed FF00 and a restart marker inside.
        j.extend(std::iter::repeat_n(0x12, donnees));
        j.extend([0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56]);
        j.extend([0xFF, 0xD9]);
        j
    }

    #[test]
    fn plus_grand_jpeg_dans_un_raw() {
        // TIFF header (little endian), orientation 6 in IFD0, then a small
        // and a large JPEG among other bytes.
        let mut raw = b"II*\0\x08\0\0\0".to_vec();
        raw.extend([0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        raw.extend([0xAB; 50]);
        let petit = jpeg(160, 120, 10);
        let grand = jpeg(1616, 1080, 500);
        raw.extend(&petit);
        raw.extend([0xFF, 0xD8, 0xFF, 0x00]); // a false start
        raw.extend(&grand);
        raw.extend([0xCD; 30]);
        let r = plus_grand_jpeg(&raw).unwrap();
        assert_eq!(&raw[r.clone()], &grand[..]);
        assert_eq!(dimensions_jpeg(&raw[r]), Some((1616, 1080)));
        assert_eq!(orientation_tiff(&raw), Some(6));
        assert_eq!(orientation_tiff(b"not a tiff"), None);
    }

    /// A real RAW file, read only (run by hand with NICEPHORE_RAW_ESSAI=<file>).
    #[test]
    #[ignore = "fichier RAW réel"]
    fn raw_reel() {
        let chemin = std::env::var("NICEPHORE_RAW_ESSAI").expect("NICEPHORE_RAW_ESSAI");
        let d = std::fs::read(&chemin).unwrap();
        let r = plus_grand_jpeg(&d).expect("aperçu intégré");
        println!("aperçu {:?} de {} octets, orientation {:?}", dimensions_jpeg(&d[r.clone()]), r.len(), orientation_tiff(&d));
        assert!(dimensions_jpeg(&d[r]).is_some_and(|(w, h)| w >= 1000 && h >= 600));
    }
}
