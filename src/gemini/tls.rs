use rustls::{
    Certificate, ClientConfig, ClientSession, DangerousClientConfig, RootCertStore,
    ServerCertVerified, ServerCertVerifier, TLSError,
};
use webpki::{DNSNameRef, InvalidDNSNameError};

use std::sync::Arc;

pub struct NoCertificateVerification {}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _roots: &RootCertStore,
        _presented_certs: &[Certificate],
        _dns_name: DNSNameRef<'_>,
        _ocsp_response: &[u8],
    ) -> Result<ServerCertVerified, TLSError> {
        // TODO: Implement TOFU
        // https://gemini.circumlunar.space/docs/tls-tutorial.gmi
        Ok(ServerCertVerified::assertion())
    }
}

pub fn client(host: &str) -> Result<ClientSession, InvalidDNSNameError> {
    let config = new_config();
    let dns_name = DNSNameRef::try_from_ascii_str(&host)?;

    Ok(ClientSession::new(&Arc::new(config), dns_name))
}

fn new_config() -> ClientConfig {
    let mut cfg = ClientConfig::new();

    let mut dangerous_config = DangerousClientConfig { cfg: &mut cfg };
    dangerous_config.set_certificate_verifier(Arc::new(NoCertificateVerification {}));

    cfg
}
