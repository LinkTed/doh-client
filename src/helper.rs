use rustls::{Certificate, RootCertStore};
use rustls_pemfile::certs;
use std::{
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind as IoErrorKind, Result as IoResult},
};

pub(super) fn load_root_store(cafile: Option<&str>) -> IoResult<RootCertStore> {
    let mut root_store = RootCertStore::empty();
    let mut added = 0;
    let mut ignored = 0;
    if let Some(cafile) = cafile {
        let cafile = File::open(&cafile)?;
        let mut cafile_buf_reader = BufReader::new(cafile);
        let cafile_der = certs(&mut cafile_buf_reader)?;
        let result = root_store.add_parsable_certificates(&cafile_der);
        added = result.0;
        ignored = result.1;
    } else if cfg!(feature = "native-certs") {
        for cert in rustls_native_certs::load_native_certs()? {
            match root_store.add(&Certificate(cert.0)) {
                Ok(_) => added += 1,
                Err(_) => ignored += 1,
            }
        }
    } else {
        panic!("feature native-certs is not enabled")
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
