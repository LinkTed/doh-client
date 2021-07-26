use rustls::RootCertStore;
use std::fs::File;
use std::io::{BufReader, Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};

pub(super) fn load_root_store(cafile: Option<&str>) -> IoResult<RootCertStore> {
    if let Some(cafile) = cafile {
        let cafile = File::open(&cafile)?;
        let mut cafile_buf_reader = BufReader::new(cafile);
        let mut root_store = RootCertStore::empty();
        if root_store.add_pem_file(&mut cafile_buf_reader).is_ok() {
            Ok(root_store)
        } else {
            Err(IoError::new(IoErrorKind::Other, "PEM parse"))
        }
    } else {
        if cfg!(feature = "native-certs") {
            match rustls_native_certs::load_native_certs() {
                Ok(root_store) => Ok(root_store),
                Err((_, e)) => Err(e),
            }
        } else {
            panic!("feature native-certs is not enabled")
        }
    }
}
