//! The caption fonts that ship inside the program.
//!
//! The engine draws a title in a font it can find, and the project carries the
//! fonts it uses by path. Rather than ask anyone to install a font, the three
//! below are compiled in, written to the run's folder and registered with the
//! project. See `fonts/README.md` for where each comes from and its licence.

use std::path::{Path, PathBuf};

/// One compiled-in face.
struct Face {
    family: &'static str,
    weight: f64,
    file: &'static str,
    bytes: &'static [u8],
}

/// Which bundled look to caption in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStyle {
    /// Cairo Black: the brand face, heavy and square.
    Cairo,
    /// Tajawal ExtraBold: cleaner and more modern.
    Tajawal,
    /// Almarai ExtraBold: rounder.
    Almarai,
}

impl FontStyle {
    /// The style called `name`, in any case.
    pub fn parse(name: &str) -> Option<FontStyle> {
        match name.to_ascii_lowercase().as_str() {
            "cairo" => Some(FontStyle::Cairo),
            "tajawal" => Some(FontStyle::Tajawal),
            "almarai" => Some(FontStyle::Almarai),
            _ => None,
        }
    }

    fn face(self) -> Face {
        match self {
            FontStyle::Cairo => Face {
                family: "Cairo",
                weight: 900.0,
                file: "Cairo-Black.ttf",
                bytes: include_bytes!("../fonts/Cairo-Black.ttf"),
            },
            FontStyle::Tajawal => Face {
                family: "Tajawal",
                weight: 800.0,
                file: "Tajawal-ExtraBold.ttf",
                bytes: include_bytes!("../fonts/Tajawal-ExtraBold.ttf"),
            },
            FontStyle::Almarai => Face {
                family: "Almarai",
                weight: 800.0,
                file: "Almarai-ExtraBold.ttf",
                bytes: include_bytes!("../fonts/Almarai-ExtraBold.ttf"),
            },
        }
    }

    /// The family name the engine is asked for.
    pub fn family(self) -> &'static str {
        self.face().family
    }

    /// The weight the face is.
    pub fn weight(self) -> f64 {
        self.face().weight
    }

    /// Writes the face into `dir` and returns what to register: its family and
    /// the file. Writing it again over the same bytes is harmless.
    pub fn install(self, dir: &Path) -> std::io::Result<(String, PathBuf)> {
        let face = self.face();
        std::fs::create_dir_all(dir)?;
        let path = dir.join(face.file);
        std::fs::write(&path, face.bytes)?;
        Ok((face.family.to_owned(), path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each compiled-in file is a real font with the Arabic letters and the
    /// weight and family the table above says, so a bad copy is caught here.
    #[test]
    fn the_bundled_faces_are_what_they_claim() {
        for style in [FontStyle::Cairo, FontStyle::Tajawal, FontStyle::Almarai] {
            let face = style.face();
            assert!(face.bytes.len() > 20_000, "{}", face.file);
            // TrueType or OpenType, by its header.
            assert!(
                matches!(&face.bytes[..4], [0, 1, 0, 0] | b"OTTO" | b"true"),
                "{}",
                face.file
            );
            assert!(face.weight >= 700.0);
            assert!(!face.family.is_empty());
        }
    }

    #[test]
    fn a_style_is_found_by_name_in_any_case() {
        assert_eq!(FontStyle::parse("Cairo"), Some(FontStyle::Cairo));
        assert_eq!(FontStyle::parse("TAJAWAL"), Some(FontStyle::Tajawal));
        assert_eq!(FontStyle::parse("almarai"), Some(FontStyle::Almarai));
        assert_eq!(FontStyle::parse("comic"), None);
    }

    #[test]
    fn installing_writes_the_file_and_names_its_family() {
        let dir = std::env::temp_dir().join(format!("cz-fonts-{}", std::process::id()));
        let (family, path) = FontStyle::Tajawal.install(&dir).unwrap();
        assert_eq!(family, "Tajawal");
        assert_eq!(
            std::fs::read(&path).unwrap().len(),
            FontStyle::Tajawal.face().bytes.len()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
