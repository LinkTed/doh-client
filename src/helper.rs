use rustls::RootCertStore;
use rustls_pemfile::{certs, read_one, Item};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind as IoErrorKind, Result as IoResult},
};

pub(super) fn load_certs(path: &str) -> IoResult<Vec<CertificateDer<'static>>> {
    let mut result = Vec::new();
    let cafile = File::open(path)?;
    let mut cafile_buf_reader = BufReader::new(cafile);
    for cert_result in certs(&mut cafile_buf_reader) {
        result.push(cert_result?);
    }
    Ok(result)
}

pub(super) fn load_private_key(path: &str) -> IoResult<PrivateKeyDer<'static>> {
    let private_key = File::open(path)?;
    let mut reader = BufReader::new(private_key);

    while let Some(item) = read_one(&mut reader)? {
        match item {
            Item::Pkcs1Key(private_key) => return Ok(PrivateKeyDer::Pkcs1(private_key)),
            Item::Pkcs8Key(private_key) => return Ok(PrivateKeyDer::Pkcs8(private_key)),
            Item::Sec1Key(private_key) => return Ok(PrivateKeyDer::Sec1(private_key)),
            _ => {}
        }
    }

    Err(IoError::new(
        IoErrorKind::Other,
        format!("Could not found any private key: {}", path),
    ))
}

#[cfg(feature = "native-certs")]
fn load_native_certs() -> (Vec<CertificateDer<'static>>, usize) {
    let result = rustls_native_certs::load_native_certs();
    (result.certs, result.errors.len())
}

#[cfg(not(feature = "native-certs"))]
fn load_native_certs() -> (Vec<CertificateDer<'static>>, usize) {
    (Vec::new(), 1)
}

pub(super) fn load_root_store(cafile: Option<&String>) -> IoResult<RootCertStore> {
    let mut root_store = RootCertStore::empty();
    let mut added = 0;
    let mut ignored = 0;
    if let Some(cafile) = cafile {
        let certs = load_certs(cafile)?;
        let result = root_store.add_parsable_certificates(certs);
        added = result.0;
        ignored = result.1;
    } else {
        let (certs, errors) = load_native_certs();
        ignored += errors;
        for cert in certs {
            match root_store.add(cert) {
                Ok(_) => added += 1,
                Err(_) => ignored += 1,
            }
        }
    }

    if added == 0 {
        Err(IoError::new(
            IoErrorKind::Other,
            "Could not add any certificates",
        ))
    } else {
        if ignored != 0 {
            error!("Could not parse all certificates: {}", ignored);
        }
        Ok(root_store)
    }
}
