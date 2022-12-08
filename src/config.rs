use std::collections::HashSet;

use miette::{IntoDiagnostic, Result};

#[derive(knuffel::Decode, Debug)]
pub struct Document {
    #[knuffel(children(name = "key-pair"))]
    pub key_pairs: Vec<KeyPair>,

    #[knuffel(children(name = "entity"))]
    pub entities: Vec<Entity>,

    #[knuffel(children(name = "certificate"))]
    pub certificates: Vec<Certificate>,
}

#[derive(knuffel::Decode, Debug)]
pub struct KeyPair {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(children)]
    pub key_type: Vec<KeyType>,
}

#[derive(knuffel::Decode, Debug)]
pub enum KeyType {
    Rsa(RsaKeyConfig),
}

#[derive(knuffel::Decode, Debug)]
pub struct RsaKeyConfig {
    #[knuffel(property, default = 2048)]
    pub num_bits: usize,
    #[knuffel(property, default = 2)]
    pub num_primes: usize,
    #[knuffel(property, default = 65537)]
    pub public_exponent: usize,
}

#[derive(knuffel::Decode, Debug)]
pub struct Entity {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub common_name: String,
    #[knuffel(children)]
    pub base_dn: Vec<EntityNameComponent>,
}

#[derive(knuffel::Decode, Debug)]
pub enum EntityNameComponent {
    CountryName(#[knuffel(argument)] String),
    StateOrProvinceName(#[knuffel(argument)] String),
    LocalityName(#[knuffel(argument)] String),
    OrganizationName(#[knuffel(argument)] String),
    OrganizationalUnitName(#[knuffel(argument)] String),
}

#[derive(knuffel::Decode, Debug)]
pub struct Certificate {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub subject_entity: String,
    #[knuffel(child, unwrap(argument))]
    pub subject_key: String,

    #[knuffel(child, unwrap(argument))]
    pub issuer_entity: String,
    #[knuffel(child, unwrap(argument))]
    pub issuer_key: String,

    #[knuffel(child, unwrap(argument))]
    pub digest_algorithm: DigestAlgorithm,

    #[knuffel(child, unwrap(argument))]
    pub not_before: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub not_after: String,

    #[knuffel(child, unwrap(argument))]
    pub serial_number: u64,

    #[knuffel(child, unwrap(children))]
    pub extensions: Vec<X509Extensions>,
}

#[derive(knuffel::DecodeScalar, Debug)]
pub enum DigestAlgorithm {
    Sha_256,
}

#[derive(knuffel::Decode, Debug)]
pub enum X509Extensions {
    BasicConstraints(BasicConstraintsExtension),
}

#[derive(knuffel::Decode, Debug)]
pub struct BasicConstraintsExtension {
    #[knuffel(property)]
    pub ca: bool,

    #[knuffel(property)]
    pub key_usage: String,
}

pub fn load_and_validate(path: &std::path::Path) -> Result<Document> {
    let in_kdl = std::fs::read_to_string(path).into_diagnostic()?;
    let doc: Document = knuffel::parse(&path.to_string_lossy(), &in_kdl)?;

    let mut kp_names: HashSet<&str> = HashSet::new();
    for kp in &doc.key_pairs {
        if kp.key_type.len() != 1 {
            miette::bail!(
                "key pairs must have exactly one key type. key pair \"{}\" has {}.",
                kp.name,
                kp.key_type.len()
            );
        }
        if !kp_names.insert(&kp.name) {
            miette::bail!(
                "key pairs must have unique names. \"{}\" is used more than once.",
                kp.name
            )
        }
    }

    let mut entity_names: HashSet<&str> = HashSet::new();
    for entity in &doc.entities {
        if !entity_names.insert(&entity.name) {
            miette::bail!(
                "entities must have unique names. \"{}\" is used more than once.",
                entity.name
            )
        }
    }

    let mut cert_names: HashSet<&str> = HashSet::new();
    for cert in &doc.certificates {
        if !cert_names.insert(&cert.name) {
            miette::bail!(
                "certificates must have unique names. \"{}\" is used more than once.",
                cert.name
            )
        }

        if let None = entity_names.get(cert.subject_entity.as_str()) {
            miette::bail!(
                "certificate \"{}\" subject entity \"{}\" does not exist",
                cert.name,
                cert.subject_key
            )
        }

        if let None = kp_names.get(cert.subject_key.as_str()) {
            miette::bail!(
                "certificate \"{}\" subject key pair \"{}\" does not exist",
                cert.name,
                cert.subject_key
            )
        }

        if let None = entity_names.get(cert.issuer_entity.as_str()) {
            miette::bail!(
                "certificate \"{}\" issuer entity \"{}\" does not exist",
                cert.name,
                cert.issuer_key
            )
        }

        if let None = kp_names.get(cert.issuer_key.as_str()) {
            miette::bail!(
                "certificate \"{}\" issuer key pair \"{}\" does not exist",
                cert.name,
                cert.issuer_key
            )
        }
    }

    Ok(doc)
}
