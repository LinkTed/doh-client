use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, read_one, Item};
use std::{
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind as IoErrorKind, Result as IoResult},
};

pub(super) fn load_certs_as_bytes(path: &str) -> IoResult<Vec<Vec<u8>>> {
    let cafile = File::open(&path)?;
    let mut cafile_buf_reader = BufReader::new(cafile);
    certs(&mut cafile_buf_reader)
}

pub(super) fn load_certs(path: &str) -> IoResult<Vec<Certificate>> {
    let certs = load_certs_as_bytes(path)?
        .iter()
        .map(|v| rustls::Certificate(v.clone()))
        .collect();
    Ok(certs)
}

pub(super) fn load_private_key(path: &str) -> IoResult<PrivateKey> {
    let private_key = File::open(&path)?;
    let mut reader = BufReader::new(private_key);

    while let Some(item) = read_one(&mut reader)? {
        match item {
            Item::RSAKey(private_key) => return Ok(PrivateKey(private_key)),
            Item::PKCS8Key(private_key) => return Ok(PrivateKey(private_key)),
            _ => {}
        }
    }

    Err(IoError::new(
        IoErrorKind::Other,
        format!("Could not found any private key: {}", path),
    ))
}

#[cfg(feature = "native-certs")]
fn load_native_certs() -> IoResult<Vec<Certificate>> {
    Ok(rustls_native_certs::load_native_certs()?
        .into_iter()
        .map(|cert| Certificate(cert.0))
        .collect())
}

#[cfg(not(feature = "native-certs"))]
fn load_native_certs() -> IoResult<Vec<Certificate>> {
    Err(IoError::new(
        IoErrorKind::Other,
        "Feature native-certs is not enabled",
    ))
}

pub(super) fn load_root_store(cafile: Option<&str>) -> IoResult<RootCertStore> {
    let mut root_store = RootCertStore::empty();
    let mut added = 0;
    let mut ignored = 0;
    if let Some(cafile) = cafile {
        let certs = load_certs_as_bytes(cafile)?;
        let result = root_store.add_parsable_certificates(&certs);
        added = result.0;
        ignored = result.1;
    } else {
        for cert in load_native_certs()? {
            match root_store.add(&cert) {
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
