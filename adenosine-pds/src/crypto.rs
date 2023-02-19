use crate::P256KeyMaterial;
use adenosine::identifiers::Did;
use anyhow::{anyhow, ensure, Result};
use p256::ecdsa::signature::{Signer, Verifier};
use std::str::FromStr;
use ucan::builder::UcanBuilder;

// Need to:
//
// - generate new random keypair
// - generate keypair from seed
// - read/write secret keypair (eg, for PDS config loading)
// - sign bytes (and ipld?) using keypair
// - verify signature bytes (and ipld?) using pubkey

const MULTICODE_P256_BYTES: [u8; 2] = [0x80, 0x24];
const MULTICODE_K256_BYTES: [u8; 2] = [0xe7, 0x01];

#[derive(Clone, PartialEq, Eq)]
pub struct KeyPair {
    public: p256::ecdsa::VerifyingKey,
    secret: p256::ecdsa::SigningKey,
}

#[derive(Clone, PartialEq, Eq)]
pub enum PubKey {
    P256(p256::ecdsa::VerifyingKey),
    K256(k256::ecdsa::VerifyingKey),
}

impl KeyPair {
    pub fn new_random() -> Self {
        let signing = p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        KeyPair {
            public: signing.verifying_key(),
            secret: signing,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<KeyPair> {
        let signing = p256::ecdsa::SigningKey::from_bytes(bytes)?;
        Ok(KeyPair {
            public: signing.verifying_key(),
            secret: signing,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.secret.to_bytes().to_vec()
    }

    pub fn pubkey(&self) -> PubKey {
        PubKey::P256(self.public)
    }

    pub fn sign_bytes(&self, data: &[u8]) -> String {
        let sig = self.secret.sign(data);
        data_encoding::BASE64URL_NOPAD.encode(&sig.to_vec())
    }

    fn ucan_keymaterial(&self) -> P256KeyMaterial {
        P256KeyMaterial(self.public, Some(self.secret.clone()))
    }

    /// This is currently just an un-validated token; we don't actually verify these.
    pub fn ucan(&self, did: &Did) -> Result<String> {
        let key_material = self.ucan_keymaterial();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(build_ucan(key_material, did))
    }

    pub fn to_hex(&self) -> String {
        data_encoding::HEXUPPER.encode(&self.to_bytes())
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        Self::from_bytes(&data_encoding::HEXUPPER.decode(hex.as_bytes())?)
    }
}

async fn build_ucan(key_material: P256KeyMaterial, did: &Did) -> Result<String> {
    let token_string = UcanBuilder::default()
        .issued_by(&key_material)
        .for_audience(did)
        .with_nonce()
        .with_lifetime(60 * 60 * 24 * 90)
        .build()?
        .sign()
        .await?
        .encode()?;
    Ok(token_string)
}

impl PubKey {
    pub fn verify_bytes(&self, data: &[u8], sig: &str) -> Result<()> {
        let sig_bytes = data_encoding::BASE64URL_NOPAD.decode(sig.as_bytes())?;
        // TODO: better way other than this re-encoding?
        let sig_hex = data_encoding::HEXUPPER.encode(&sig_bytes);
        match self {
            PubKey::P256(key) => {
                let sig = p256::ecdsa::Signature::from_str(&sig_hex)?;
                Ok(key.verify(data, &sig)?)
            }
            PubKey::K256(key) => {
                let sig = k256::ecdsa::Signature::from_str(&sig_hex)?;
                Ok(key.verify(data, &sig)?)
            }
        }
    }

    pub fn key_type(&self) -> String {
        match self {
            PubKey::P256(_) => "EcdsaSecp256r1VerificationKey2019",
            PubKey::K256(_) => "EcdsaSecp256k1VerificationKey2019",
        }
        .to_string()
    }

    /// This public verification key encoded as base58btc multibase string, not 'compressed', as
    /// included in DID documents ('publicKeyMultibase').
    ///
    /// Note that the did:key serialization does 'compress' the key into a smaller size.
    pub fn to_multibase(&self) -> String {
        let mut bytes: Vec<u8> = vec![];
        match self {
            PubKey::P256(key) => {
                bytes.extend_from_slice(&MULTICODE_P256_BYTES);
                bytes.extend_from_slice(&key.to_encoded_point(false).to_bytes());
            }
            PubKey::K256(key) => {
                bytes.extend_from_slice(&MULTICODE_K256_BYTES);
                bytes.extend_from_slice(&key.to_bytes());
            }
        }
        multibase::encode(multibase::Base::Base58Btc, &bytes)
    }

    /// Serializes as a 'did:key' string.
    pub fn to_did_key(&self) -> String {
        let mut bytes: Vec<u8> = vec![];
        match self {
            PubKey::P256(key) => {
                bytes.extend_from_slice(&MULTICODE_P256_BYTES);
                bytes.extend_from_slice(&key.to_encoded_point(true).to_bytes());
            }
            PubKey::K256(key) => {
                bytes.extend_from_slice(&MULTICODE_K256_BYTES);
                bytes.extend_from_slice(&key.to_bytes());
            }
        }
        format!(
            "did:key:{}",
            multibase::encode(multibase::Base::Base58Btc, &bytes)
        )
    }

    pub fn from_did_key(did_key: &str) -> Result<Self> {
        if !did_key.starts_with("did:key:") || did_key.len() < 20 {
            return Err(anyhow!("does not look like a did:key: {}", did_key));
        }
        let (key_type, bytes) = multibase::decode(&did_key[8..])?;
        ensure!(
            key_type == multibase::Base::Base58Btc,
            "base58btc-encoded key"
        );
        // prefix bytes
        let prefix: [u8; 2] = [bytes[0], bytes[1]];
        match prefix {
            MULTICODE_K256_BYTES => Ok(PubKey::K256(k256::ecdsa::VerifyingKey::from_sec1_bytes(
                &bytes[2..],
            )?)),
            MULTICODE_P256_BYTES => Ok(PubKey::P256(p256::ecdsa::VerifyingKey::from_sec1_bytes(
                &bytes[2..],
            )?)),
            _ => Err(anyhow!(
                "key type (multicodec) not handled when parsing DID key: {}",
                did_key
            )),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PubKey::P256(key) => key.to_encoded_point(true).to_bytes().to_vec(),
            PubKey::K256(key) => key.to_bytes().to_vec(),
        }
    }

    pub fn ucan_keymaterial(&self) -> P256KeyMaterial {
        match self {
            PubKey::P256(key) => P256KeyMaterial(*key, None),
            PubKey::K256(_key) => unimplemented!(),
        }
    }
}

impl std::fmt::Display for PubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_did_key())
    }
}

#[test]
fn test_did_secp256k1_p256() {
    // did:key secp256k1 test vectors from W3C
    // https://github.com/w3c-ccg/did-method-key/blob/main/test-vectors/secp256k1.json
    // via atproto repo
    let pairs = vec![
        (
            "9085d2bef69286a6cbb51623c8fa258629945cd55ca705cc4e66700396894e0c",
            "did:key:zQ3shokFTS3brHcDQrn82RUDfCZESWL1ZdCEJwekUDPQiYBme",
        ),
        (
            "f0f4df55a2b3ff13051ea814a8f24ad00f2e469af73c363ac7e9fb999a9072ed",
            "did:key:zQ3shtxV1FrJfhqE1dvxYRcCknWNjHc3c5X1y3ZSoPDi2aur2",
        ),
        (
            "6b0b91287ae3348f8c2f2552d766f30e3604867e34adc37ccbb74a8e6b893e02",
            "did:key:zQ3shZc2QzApp2oymGvQbzP8eKheVshBHbU4ZYjeXqwSKEn6N",
        ),
        (
            "c0a6a7c560d37d7ba81ecee9543721ff48fea3e0fb827d42c1868226540fac15",
            "did:key:zQ3shadCps5JLAHcZiuX5YUtWHHL8ysBJqFLWvjZDKAWUBGzy",
        ),
        (
            "175a232d440be1e0788f25488a73d9416c04b6f924bea6354bf05dd2f1a75133",
            "did:key:zQ3shptjE6JwdkeKN4fcpnYQY3m9Cet3NiHdAfpvSUZBFoKBj",
        ),
    ];

    // test decode/encode did:key
    for (_hex, did) in pairs.iter() {
        assert_eq!(did, &PubKey::from_did_key(did).unwrap().to_did_key());
    }

    let p256_dids = vec![
        "did:key:zDnaerx9CtbPJ1q36T5Ln5wYt3MQYeGRG5ehnPAmxcf5mDZpv",
        "did:key:zDnaerDaTF5BXEavCrfRZEk316dpbLsfPDZ3WJ5hRTPFU2169",
    ];
    for did in p256_dids {
        assert_eq!(did, &PubKey::from_did_key(did).unwrap().to_did_key());
    }
}

#[test]
fn test_did_plc_examples() {
    // https://atproto.com/specs/did-plc
    let example_dids = vec![
        "did:key:zDnaejYFhgFiVF89LhJ4UipACLKuqo6PteZf8eKDVKeExXUPk",
        "did:key:zDnaeSezF2TgCD71b5DiiFyhHQwKAfsBVqTTHRMvP597Z5Ztn",
        "did:key:zDnaeh9v2RmcMo13Du2d6pjUf5bZwtauYxj3n9dYjw4EZUAR7",
        "did:key:zDnaedvvAsDE6H3BDdBejpx9ve2Tz95cymyCAKF66JbyMh1Lt",
    ];

    for did in example_dids {
        assert_eq!(did, &PubKey::from_did_key(did).unwrap().to_did_key());
    }
}

#[test]
fn test_signing() {
    let msg = b"you have found the secret message";
    let keypair = KeyPair::new_random();
    let sig_str = keypair.sign_bytes(msg);
    keypair.pubkey().verify_bytes(msg, &sig_str).unwrap();

    // and with pubkey that has been serialized/deserialized
    let did_key = keypair.pubkey().to_did_key();
    let pubkey = PubKey::from_did_key(&did_key).unwrap();
    pubkey.verify_bytes(msg, &sig_str).unwrap();
}

#[test]
fn test_keypair_hex() {
    let before = KeyPair::new_random();
    let after = KeyPair::from_hex(&before.to_hex()).unwrap();
    assert!(before == after);
}
