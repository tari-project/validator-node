use super::errors::TypeError;
use core::cmp::PartialEq;
use std::{
    fmt,
    hash::{Hash, Hasher},
};

/// Tari uses templates to define the behaviour for its smart contracts.
/// The [Template ID](https://rfc.tari.com/RFC-0311_AssetTemplates.html#template-id)
/// refers to the type of digital asset being created.
#[derive(Clone, Copy)]
pub struct TemplateID {
    template_type: u32,
    template_version: u16,
    beta: bool,
    confidential: bool,
}
impl fmt::Display for TemplateID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.template_type, self.template_version)?;
        if self.beta {
            write!(f, "-beta")?;
        }
        if self.confidential {
            write!(f, "-confidential")?;
        }
        Ok(())
    }
}

/// Only template type and template version take part in comparison
impl PartialEq for TemplateID {
    fn eq(&self, other: &Self) -> bool {
        self.template_type == other.template_type && self.template_version == other.template_version
    }
}

/// Only template type and template version take part in hashing
impl Hash for TemplateID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template_type.hash(state);
        self.template_version.hash(state);
    }
}

impl TemplateID {
    pub fn new(template_type: u32, template_version: u16, beta: bool, confidential: bool) -> Self {
        Self {
            template_type,
            template_version,
            beta,
            confidential,
        }
    }

    /// Template type (0 - 4,294,967,295)
    #[inline]
    pub fn template_type(&self) -> u32 {
        self.template_type
    }

    /// Template version (0 - 65,535)
    #[inline]
    pub fn template_version(&self) -> u16 {
        self.template_version
    }

    /// Beta Mode flag
    #[inline]
    pub fn beta(&self) -> bool {
        self.beta
    }

    /// Confidentiality flag
    #[inline]
    pub fn confidential(&self) -> bool {
        self.confidential
    }

    /// Template type as 8-byte hex
    #[inline]
    pub fn type_hex(&self) -> String {
        format!("{:X}", self.template_type)
    }

    /// Template version as 4-byte hex
    #[inline]
    pub fn version_hex(&self) -> String {
        format!("{:X}", self.template_version)
    }

    /// Convert from 12-char hex, considering beta and confidential is false
    #[inline]
    pub fn to_hex(&self) -> String {
        format!("{:X}{:X}", self.template_version, self.template_type)
    }

    /// Convert from 12-char hex, considering beta and confidential is false
    pub fn from_hex(hex: &str) -> Result<Self, TypeError> {
        if hex.len() != 12 {
            return Err(TypeError::parse(format!(
                "TemplateID expected 12-char hex string, got {}",
                hex.len()
            )));
        }
        let template_type = u32::from_str_radix(&hex[0..8], 16)
            .map_err(|err| TypeError::parse_field("TemplateID::type", err.into()))?;
        let template_version = u16::from_str_radix(&hex[8..12], 16)
            .map_err(|err| TypeError::parse_field("TemplateID::version", err.into()))?;
        Ok(Self {
            template_type,
            template_version,
            beta: false,
            confidential: false,
        })
    }
}
