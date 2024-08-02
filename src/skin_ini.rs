use ini::{Ini, ParseError};
use thiserror::Error;
use std::io;

use crate::rgb::Rgb;

#[derive(Error, Debug)]
pub enum SkinParseError {
    #[error("parsing .ini file: `{0}`")]
    ParseError(#[from] ini::Error),
    #[error("couldn't find a required field `{0}`")]
    MissingRequiredField(String)
}

impl SkinParseError {
    pub fn field(name: &str) -> Self {
        Self::MissingRequiredField(name.to_owned())
    }
}

#[derive(Debug, Default)]
struct Colours {
    colour1: Option<Rgb>,
    colour2: Option<Rgb>,
    colour3: Option<Rgb>,
    colour4: Option<Rgb>,
    colour5: Option<Rgb>,
    colour6: Option<Rgb>,
    colour7: Option<Rgb>,
    colour8: Option<Rgb>,
}

#[derive(Debug)]
struct General {
    pub name: String,
    pub author: String
}

#[derive(Debug)]
pub struct SkinIni {
    pub general: General,
    pub colours: Colours,
}

impl SkinIni {
    pub fn parse(bytes: &[u8]) -> Result<Self, SkinParseError> {
        let ini = Ini::read_from(&mut io::Cursor::new(bytes))?;

        // General
        let name = ini.get_from(Some("General"), "Name").ok_or(SkinParseError::field("Name"))?;
        let author = ini.get_from(Some("General"), "Author").ok_or(SkinParseError::field("Author"))?;

        let general = General {
            name: name.to_owned(), 
            author: author.to_owned(),
        };

        // Colours
        // TODO write some cool macro here?
        let colour1 = ini.get_from(Some("Colours"), "Combo1").map_or(None, |c| Rgb::parse(c));
        let colour2 = ini.get_from(Some("Colours"), "Combo2").map_or(None, |c| Rgb::parse(c));
        let colour3 = ini.get_from(Some("Colours"), "Combo3").map_or(None, |c| Rgb::parse(c));
        let colour4 = ini.get_from(Some("Colours"), "Combo4").map_or(None, |c| Rgb::parse(c));
        let colour5 = ini.get_from(Some("Colours"), "Combo5").map_or(None, |c| Rgb::parse(c));
        let colour6 = ini.get_from(Some("Colours"), "Combo6").map_or(None, |c| Rgb::parse(c));
        let colour7 = ini.get_from(Some("Colours"), "Combo7").map_or(None, |c| Rgb::parse(c));
        let colour8 = ini.get_from(Some("Colours"), "Combo8").map_or(None, |c| Rgb::parse(c));

        let colours = Colours {
            colour1,
            colour2,
            colour3,
            colour4,
            colour5,
            colour6,
            colour7,
            colour8,
        };

        Ok(Self {
            general,
            colours,
        })
    }
}

impl Default for SkinIni {
    fn default() -> Self {
        let general = General {
            name: "Default".to_owned(),
            author: "486c".to_owned()
        };

        let colours = Colours {
            colour1: Some(Rgb::new(255, 255, 255)),
            ..Default::default()
        };

        Self {
            colours,
            general,
        }
    }
}

#[test]
fn test_color_parse() {
    let s = "254, 255, 255";
    let parsed = Rgb::parse(s).unwrap();
    assert_eq!(parsed.r(), 254);
    assert_eq!(parsed.g(), 255);
    assert_eq!(parsed.b(), 255);

    let s = "254,  255,  10   ";
    let parsed = Rgb::parse(s).unwrap();
    assert_eq!(parsed.r(), 254);
    assert_eq!(parsed.g(), 255);
    assert_eq!(parsed.b(), 10);
}
