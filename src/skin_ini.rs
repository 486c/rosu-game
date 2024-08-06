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

#[derive(Debug)]
pub struct Colours {
    pub combo_colors: Vec<Rgb>,
    pub slider_border: Rgb,
    pub slider_body: Rgb,
}

#[derive(Debug)]
pub struct General {
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
        let slider_border = Rgb::parse(ini.get_from(Some("Colours"), "SliderBorder").unwrap_or("255, 255, 255"))
            .unwrap();

        let slider_body = Rgb::parse(ini.get_from(Some("Colours"), "SliderTrackOverride").unwrap_or("0, 0, 0"))
            .unwrap();


        //SliderTrackOverride

        // TODO write some cool macro here?
        let mut colors = Vec::new();

        let colour1 = ini.get_from(Some("Colours"), "Combo1").map_or(None, |c| Rgb::parse(c));
        let colour2 = ini.get_from(Some("Colours"), "Combo2").map_or(None, |c| Rgb::parse(c));
        let colour3 = ini.get_from(Some("Colours"), "Combo3").map_or(None, |c| Rgb::parse(c));
        let colour4 = ini.get_from(Some("Colours"), "Combo4").map_or(None, |c| Rgb::parse(c));
        let colour5 = ini.get_from(Some("Colours"), "Combo5").map_or(None, |c| Rgb::parse(c));
        let colour6 = ini.get_from(Some("Colours"), "Combo6").map_or(None, |c| Rgb::parse(c));
        let colour7 = ini.get_from(Some("Colours"), "Combo7").map_or(None, |c| Rgb::parse(c));
        let colour8 = ini.get_from(Some("Colours"), "Combo8").map_or(None, |c| Rgb::parse(c));

        if let Some(c) = colour1 {
            colors.push(c);
        }

        if let Some(c) = colour2 {
            colors.push(c);
        }

        if let Some(c) = colour3 {
            colors.push(c);
        }

        if let Some(c) = colour4 {
            colors.push(c);
        }

        if let Some(c) = colour5 {
            colors.push(c);
        }

        if let Some(c) = colour6 {
            colors.push(c);
        }

        if let Some(c) = colour7 {
            colors.push(c);
        }

        if let Some(c) = colour8 {
            colors.push(c);
        }

        if colors.is_empty() {
            colors.push(Rgb::new(255, 255, 255))
        }

        let colours = Colours {
            slider_border,
            slider_body,
            combo_colors: colors,
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
            combo_colors: vec![Rgb::new(255, 255, 255)],
            slider_border: Rgb::new(255, 255, 255),
            slider_body: Rgb::new(0, 0, 0),
        };

        Self {
            colours,
            general,
        }
    }
}
