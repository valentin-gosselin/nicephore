//! EXIF orientation of a JPEG, written without touching its pixels: a photo
//! turned in the viewer only has its orientation tag changed (lossless,
//! instant), as most photo viewers do.

/// Orientation tag of IFD0.
const ORIENTATION: u16 = 0x0112;
/// TIFF type SHORT.
const SHORT: u16 = 3;

/// An EXIF orientation (1 to 8) as a horizontal mirror applied first, then
/// a number of quarter turns clockwise.
fn decomposer(orientation: u16) -> (bool, u8) {
    match orientation {
        2 => (true, 0),
        3 => (false, 2),
        4 => (true, 2),
        5 => (true, 3),
        6 => (false, 1),
        7 => (true, 1),
        8 => (false, 3),
        _ => (false, 0),
    }
}

fn composer(miroir: bool, quarts: u8) -> u16 {
    match (miroir, quarts % 4) {
        (false, 0) => 1,
        (false, 1) => 6,
        (false, 2) => 3,
        (false, 3) => 8,
        (true, 0) => 2,
        (true, 1) => 7,
        (true, 2) => 4,
        _ => 5,
    }
}

/// The orientation of a photo shown with `orientation` then turned by
/// `quarts` quarter turns clockwise.
pub fn tournee(orientation: u16, quarts: u8) -> u16 {
    let (miroir, q) = decomposer(orientation);
    composer(miroir, q + quarts)
}

struct Tiff<'a> {
    octets: &'a [u8],
    motorola: bool,
}

impl Tiff<'_> {
    fn u16(&self, i: usize) -> Option<u16> {
        let b: [u8; 2] = self.octets.get(i..i + 2)?.try_into().ok()?;
        Some(if self.motorola { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
    }
    fn u32(&self, i: usize) -> Option<u32> {
        let b: [u8; 4] = self.octets.get(i..i + 4)?.try_into().ok()?;
        Some(if self.motorola { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    }
    fn ecrire_u16(&self, v: u16) -> [u8; 2] {
        if self.motorola { v.to_be_bytes() } else { v.to_le_bytes() }
    }
    fn ecrire_u32(&self, v: u32) -> [u8; 4] {
        if self.motorola { v.to_be_bytes() } else { v.to_le_bytes() }
    }
}

/// Where the EXIF APP1 segment of a JPEG is: start of the marker, and the
/// end of the markers before the image data (where a new one may go).
struct Segments {
    exif: Option<(usize, usize)>,
    /// After SOI, or after a JFIF APP0 (which must stay first).
    insertion: usize,
}

fn segments(jpeg: &[u8]) -> Result<Segments, String> {
    if jpeg.get(..2) != Some(&[0xFF, 0xD8]) {
        return Err("ce fichier n'est pas un JPEG".into());
    }
    let mut i = 2;
    let mut exif = None;
    let mut insertion = 2;
    loop {
        // Fill bytes before a marker are allowed.
        while jpeg.get(i) == Some(&0xFF) && jpeg.get(i + 1) == Some(&0xFF) {
            i += 1;
        }
        let (Some(&0xFF), Some(&marqueur)) = (jpeg.get(i), jpeg.get(i + 1)) else {
            return Err("JPEG illisible (marqueur attendu)".into());
        };
        // Start of scan or end of image: no metadata after.
        if marqueur == 0xDA || marqueur == 0xD9 {
            break;
        }
        let longueur = jpeg.get(i + 2..i + 4).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize).ok_or("JPEG tronqué")?;
        if longueur < 2 || i + 2 + longueur > jpeg.len() {
            return Err("JPEG tronqué".into());
        }
        let donnees = &jpeg[i + 4..i + 2 + longueur];
        if marqueur == 0xE1 && exif.is_none() && donnees.starts_with(b"Exif\0\0") {
            exif = Some((i, i + 2 + longueur));
        }
        if marqueur == 0xE0 && insertion == 2 && i == 2 {
            insertion = i + 2 + longueur;
        }
        i += 2 + longueur;
    }
    Ok(Segments { exif, insertion })
}

/// Orientation written in the JPEG (1 when it has none).
pub fn lire(jpeg: &[u8]) -> Result<u16, String> {
    let s = segments(jpeg)?;
    let Some((debut, fin)) = s.exif else { return Ok(1) };
    let tiff = Tiff { octets: &jpeg[debut + 10..fin], motorola: jpeg.get(debut + 10) == Some(&b'M') };
    let ifd0 = tiff.u32(4).ok_or("EXIF illisible")? as usize;
    let n = tiff.u16(ifd0).ok_or("EXIF illisible")? as usize;
    for e in 0..n {
        let entree = ifd0 + 2 + e * 12;
        if tiff.u16(entree) == Some(ORIENTATION) {
            return Ok(tiff.u16(entree + 8).filter(|o| (1..=8).contains(o)).unwrap_or(1));
        }
    }
    Ok(1)
}

/// The JPEG with its orientation set to `orientation`: the tag rewritten in
/// place when there is one, otherwise added (in a new IFD0, or a new EXIF
/// segment). The pixels are never touched.
pub fn ecrire(jpeg: &[u8], orientation: u16) -> Result<Vec<u8>, String> {
    let s = segments(jpeg)?;
    let Some((debut, fin)) = s.exif else {
        if orientation == 1 {
            return Ok(jpeg.to_vec());
        }
        // A minimal EXIF: big-endian TIFF, one IFD0 holding the orientation.
        let mut tiff: Vec<u8> = b"MM\0\x2a\0\0\0\x08".to_vec();
        tiff.extend_from_slice(&1u16.to_be_bytes());
        tiff.extend_from_slice(&ORIENTATION.to_be_bytes());
        tiff.extend_from_slice(&SHORT.to_be_bytes());
        tiff.extend_from_slice(&1u32.to_be_bytes());
        tiff.extend_from_slice(&orientation.to_be_bytes());
        tiff.extend_from_slice(&[0, 0]);
        tiff.extend_from_slice(&0u32.to_be_bytes());
        let mut sortie = Vec::with_capacity(jpeg.len() + tiff.len() + 10);
        sortie.extend_from_slice(&jpeg[..s.insertion]);
        sortie.extend_from_slice(&[0xFF, 0xE1]);
        sortie.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        sortie.extend_from_slice(b"Exif\0\0");
        sortie.extend_from_slice(&tiff);
        sortie.extend_from_slice(&jpeg[s.insertion..]);
        return Ok(sortie);
    };
    let base_tiff = debut + 10;
    let tiff = Tiff { octets: &jpeg[base_tiff..fin], motorola: jpeg.get(base_tiff) == Some(&b'M') };
    if tiff.u16(2) != Some(42) {
        return Err("EXIF illisible".into());
    }
    let ifd0 = tiff.u32(4).ok_or("EXIF illisible")? as usize;
    let n = tiff.u16(ifd0).ok_or("EXIF illisible")? as usize;
    let suivante = tiff.u32(ifd0 + 2 + n * 12).ok_or("EXIF illisible")?;
    let mut entrees: Vec<[u8; 12]> = Vec::with_capacity(n + 1);
    for e in 0..n {
        let i = ifd0 + 2 + e * 12;
        let entree: [u8; 12] = tiff.octets[i..i + 12].try_into().map_err(|_| "EXIF illisible")?;
        if tiff.u16(i) == Some(ORIENTATION) {
            // In place: same size, every offset of the file unchanged.
            let mut sortie = jpeg.to_vec();
            let valeur = base_tiff + i + 8;
            if tiff.u16(i + 2) != Some(SHORT) {
                return Err("étiquette d'orientation d'un type inattendu".into());
            }
            sortie[valeur..valeur + 2].copy_from_slice(&tiff.ecrire_u16(orientation));
            return Ok(sortie);
        }
        entrees.push(entree);
    }
    if orientation == 1 {
        return Ok(jpeg.to_vec());
    }
    // No orientation yet: IFD0 copied after the EXIF data with one more
    // entry. Its other entries point where they did (offsets are from the
    // start of the TIFF data), the old IFD0 is just left unused.
    let mut nouvelle = [0u8; 12];
    nouvelle[0..2].copy_from_slice(&tiff.ecrire_u16(ORIENTATION));
    nouvelle[2..4].copy_from_slice(&tiff.ecrire_u16(SHORT));
    nouvelle[4..8].copy_from_slice(&tiff.ecrire_u32(1));
    nouvelle[8..10].copy_from_slice(&tiff.ecrire_u16(orientation));
    let place = entrees.iter().position(|e| tiff.motorola_u16(&e[0..2]) > ORIENTATION).unwrap_or(entrees.len());
    entrees.insert(place, nouvelle);
    let mut donnees_tiff = tiff.octets.to_vec();
    if donnees_tiff.len() % 2 == 1 {
        donnees_tiff.push(0);
    }
    let decalage = donnees_tiff.len() as u32;
    donnees_tiff.extend_from_slice(&tiff.ecrire_u16(entrees.len() as u16));
    for e in &entrees {
        donnees_tiff.extend_from_slice(e);
    }
    donnees_tiff.extend_from_slice(&tiff.ecrire_u32(suivante));
    let ecrit = tiff.ecrire_u32(decalage);
    donnees_tiff[4..8].copy_from_slice(&ecrit);
    let longueur = donnees_tiff.len() + 8;
    if longueur > u16::MAX as usize {
        return Err("EXIF trop grand pour y ajouter l'orientation".into());
    }
    let mut sortie = Vec::with_capacity(jpeg.len() + 20 + entrees.len() * 12);
    sortie.extend_from_slice(&jpeg[..debut]);
    sortie.extend_from_slice(&[0xFF, 0xE1]);
    sortie.extend_from_slice(&(longueur as u16).to_be_bytes());
    sortie.extend_from_slice(b"Exif\0\0");
    sortie.extend_from_slice(&donnees_tiff);
    sortie.extend_from_slice(&jpeg[fin..]);
    Ok(sortie)
}

impl Tiff<'_> {
    fn motorola_u16(&self, b: &[u8]) -> u16 {
        let b = [b[0], b[1]];
        if self.motorola { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_sans_exif() -> Vec<u8> {
        let image = image::RgbImage::from_fn(8, 4, |x, _| image::Rgb([if x < 4 { 255 } else { 0 }, 0, 0]));
        let mut tampon = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image).write_to(&mut tampon, image::ImageFormat::Jpeg).unwrap();
        tampon.into_inner()
    }

    /// Orientation as read by another library (kamadak-exif).
    fn orientation_exif(jpeg: &[u8]) -> Option<u32> {
        let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(jpeg)).ok()?;
        exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?.value.get_uint(0)
    }

    #[test]
    fn rotations_composees() {
        assert_eq!(tournee(1, 1), 6);
        assert_eq!(tournee(6, 1), 3);
        assert_eq!(tournee(3, 1), 8);
        assert_eq!(tournee(8, 1), 1);
        assert_eq!(tournee(6, 3), 1);
        // Mirrored: the mirror stays, the turn adds up.
        assert_eq!(tournee(2, 1), 7);
        assert_eq!(tournee(4, 2), 2);
        for o in 1..=8 {
            assert_eq!(tournee(tournee(o, 1), 3), o);
        }
    }

    #[test]
    fn ajout_puis_reecriture_en_place() {
        let brut = jpeg_sans_exif();
        assert_eq!(lire(&brut).unwrap(), 1);
        let avec = ecrire(&brut, 6).unwrap();
        assert_eq!(lire(&avec).unwrap(), 6);
        assert_eq!(orientation_exif(&avec), Some(6));
        let encore = ecrire(&avec, 3).unwrap();
        assert_eq!(encore.len(), avec.len(), "réécrite en place");
        assert_eq!(orientation_exif(&encore), Some(3));
        // The image itself still decodes, same size.
        let decodee = image::load_from_memory(&encore).unwrap();
        assert_eq!((decodee.width(), decodee.height()), (8, 4));
        // Nothing to do for "upright" without EXIF.
        assert_eq!(ecrire(&brut, 1).unwrap(), brut);
    }

    #[test]
    fn exif_sans_orientation_complete() {
        // An EXIF with only a "Make" tag (ASCII, inline), little-endian.
        let brut = jpeg_sans_exif();
        let mut tiff: Vec<u8> = b"II\x2a\0\x08\0\0\0".to_vec();
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x010Fu16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&4u32.to_le_bytes());
        tiff.extend_from_slice(b"Abc\0");
        tiff.extend_from_slice(&0u32.to_le_bytes());
        let mut jpeg = brut[..2].to_vec();
        jpeg.extend_from_slice(&[0xFF, 0xE1]);
        jpeg.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        jpeg.extend_from_slice(b"Exif\0\0");
        jpeg.extend_from_slice(&tiff);
        jpeg.extend_from_slice(&brut[2..]);
        assert_eq!(lire(&jpeg).unwrap(), 1);
        let tournee = ecrire(&jpeg, 8).unwrap();
        assert_eq!(orientation_exif(&tournee), Some(8));
        let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&tournee)).unwrap();
        let marque = exif.get_field(exif::Tag::Make, exif::In::PRIMARY).unwrap();
        assert_eq!(marque.display_value().to_string(), "\"Abc\"");
        assert!(image::load_from_memory(&tournee).is_ok());
    }
}
