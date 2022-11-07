use crate::{KeyPair, PubKey};
/// DID and 'did:plc' stuff
///
/// This is currently a partial/skeleton implementation, which only generates local/testing did:plc
/// DIDs (and DID documents) using a single 'create' genesis block. Key rotation, etc, is not
/// supported.
use anyhow::Result;
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;
use libipld::{Block, Cid, DagCbor, DefaultParams};
use serde_json::json;

#[allow(non_snake_case)]
#[derive(Debug, DagCbor, PartialEq, Eq, Clone)]
pub struct CreateOp {
    #[ipld(rename = "type")]
    pub op_type: String,
    pub signingKey: String,
    pub recoveryKey: String,
    pub username: String,
    pub service: String,
    pub prev: Option<Cid>,
    pub sig: String,
}

#[allow(non_snake_case)]
#[derive(Debug, DagCbor, PartialEq, Eq, Clone)]
struct UnsignedCreateOp {
    #[ipld(rename = "type")]
    pub op_type: String,
    pub signingKey: String,
    pub recoveryKey: String,
    pub username: String,
    pub service: String,
    pub prev: Option<Cid>,
}

impl UnsignedCreateOp {
    fn into_signed(self, sig: String) -> CreateOp {
        CreateOp {
            op_type: self.op_type,
            prev: self.prev,
            sig: sig,
            signingKey: self.signingKey,
            recoveryKey: self.recoveryKey,
            username: self.username,
            service: self.service,
        }
    }
}

impl CreateOp {
    pub fn new(
        username: String,
        atp_pds: String,
        keypair: &KeyPair,
        recovery_key: Option<String>,
    ) -> Self {
        let signing_key = keypair.pubkey().to_did_key();
        let recovery_key = recovery_key.unwrap_or(signing_key.clone());
        let unsigned = UnsignedCreateOp {
            op_type: "create".to_string(),
            prev: None,
            signingKey: signing_key,
            recoveryKey: recovery_key,
            username: username,
            service: atp_pds,
        };
        let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, &unsigned)
            .expect("encode DAG-CBOR");
        let sig = keypair.sign_bytes(block.data());
        unsigned.into_signed(sig)
    }

    pub fn did_plc(&self) -> String {
        // dump DAG-CBOR
        let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, self)
            .expect("encode DAG-CBOR");
        let bin = block.data();
        // hash SHA-256
        let digest_bytes: Vec<u8> = data_encoding::HEXLOWER
            .decode(sha256::digest(bin).as_bytes())
            .expect("SHA-256 digest is always hex string");
        // encode base32
        let digest_b32 = data_encoding::BASE32_NOPAD
            .encode(&digest_bytes)
            .to_ascii_lowercase();
        // truncate
        format!("did:plc:{}", &digest_b32[0..24])
    }

    pub fn did_doc(&self) -> serde_json::Value {
        let did = self.did_plc();
        // TODO:
        let user_url = format!("https://{}.test", self.username);
        let key_type = "EcdsaSecp256r1VerificationKey2019";
        json!({
            "@context": [
                "https://www.w3.org/ns/did/v1",
                "https://w3id.org/security/suites/ecdsa-2019/v1"
            ],
            "id": did,
            "alsoKnownAs": [ user_url ],
            "verificationMethod": [
                {
                "id": format!("{}#signingKey)", did),
                "type": key_type,
                "controller": did,
                "publicKeyMultibase": self.signingKey
                },
                {
                "id": format!("{}#recoveryKey)", did),
                "type": key_type,
                "controller": did,
                "publicKeyMultibase": self.recoveryKey
                }
            ],
            "assertionMethod": [ format!("{}#signingKey)", did)],
            "capabilityInvocation": [ format!("{}#signingKey)", did) ],
            "capabilityDelegation": [ format!("{}#signingKey)", did) ],
            "service": [
                {
                "id": format!("{}#atpPds)", did),
                "type": "AtpPersonalDataServer",
                "serviceEndpoint": self.service
                }
            ]
        })
    }

    fn into_unsigned(self) -> UnsignedCreateOp {
        UnsignedCreateOp {
            op_type: self.op_type,
            prev: self.prev,
            signingKey: self.signingKey,
            recoveryKey: self.recoveryKey,
            username: self.username,
            service: self.service,
        }
    }

    /// This method only makes sense on the "genesis" create object
    pub fn verify_self(&self) -> Result<()> {
        let key = PubKey::from_did_key(&self.signingKey)?;
        let unsigned = {
            let cpy = (*self).clone();
            cpy.into_unsigned()
        };
        //println!("unsigned: {:?}", unsigned);
        let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, &unsigned)
            .expect("encode DAG-CBOR");
        key.verify_bytes(block.data(), &self.sig)
    }
}

#[test]
fn test_debug_did_signing() {
    let op = UnsignedCreateOp {
        op_type: "create".to_string(),
        signingKey: "did:key:zDnaeSWVQyW8DSF6mDwT9j8YrzDWDs8h6PPjuTcipzG84iCBE".to_string(),
        recoveryKey: "did:key:zDnaeSWVQyW8DSF6mDwT9j8YrzDWDs8h6PPjuTcipzG84iCBE".to_string(),
        username: "carla.test".to_string(),
        service: "http://localhost:2583".to_string(),
        prev: None,
    };
    let block =
        Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, &op).expect("encode DAG-CBOR");
    let op_bytes = block.data();

    let _key_bytes = vec![
        4, 30, 224, 8, 198, 84, 108, 1, 58, 193, 91, 176, 212, 45, 4, 36, 28, 252, 242, 95, 20, 85,
        87, 246, 79, 134, 42, 113, 5, 216, 238, 235, 21, 146, 16, 88, 239, 217, 36, 252, 148, 197,
        203, 22, 29, 2, 52, 152, 77, 208, 21, 88, 2, 85, 219, 212, 148, 139, 104, 200, 15, 119, 46,
        178, 186,
    ];

    let pub_key =
        PubKey::from_did_key("did:key:zDnaeSWVQyW8DSF6mDwT9j8YrzDWDs8h6PPjuTcipzG84iCBE").unwrap();
    //let keypair = KeyPair::from_bytes(&key_bytes).unwrap();
    //assert_eq!(keypair.to_bytes(), key_bytes);

    let encoded_bytes = vec![
        166, 100, 112, 114, 101, 118, 246, 100, 116, 121, 112, 101, 102, 99, 114, 101, 97, 116,
        101, 103, 115, 101, 114, 118, 105, 99, 101, 117, 104, 116, 116, 112, 58, 47, 47, 108, 111,
        99, 97, 108, 104, 111, 115, 116, 58, 50, 53, 56, 51, 104, 117, 115, 101, 114, 110, 97, 109,
        101, 106, 99, 97, 114, 108, 97, 46, 116, 101, 115, 116, 106, 115, 105, 103, 110, 105, 110,
        103, 75, 101, 121, 120, 57, 100, 105, 100, 58, 107, 101, 121, 58, 122, 68, 110, 97, 101,
        83, 87, 86, 81, 121, 87, 56, 68, 83, 70, 54, 109, 68, 119, 84, 57, 106, 56, 89, 114, 122,
        68, 87, 68, 115, 56, 104, 54, 80, 80, 106, 117, 84, 99, 105, 112, 122, 71, 56, 52, 105, 67,
        66, 69, 107, 114, 101, 99, 111, 118, 101, 114, 121, 75, 101, 121, 120, 57, 100, 105, 100,
        58, 107, 101, 121, 58, 122, 68, 110, 97, 101, 83, 87, 86, 81, 121, 87, 56, 68, 83, 70, 54,
        109, 68, 119, 84, 57, 106, 56, 89, 114, 122, 68, 87, 68, 115, 56, 104, 54, 80, 80, 106,
        117, 84, 99, 105, 112, 122, 71, 56, 52, 105, 67, 66, 69,
    ];
    assert_eq!(encoded_bytes, op_bytes);

    let _sig_bytes = vec![
        131, 115, 47, 143, 89, 68, 79, 73, 121, 198, 70, 76, 91, 64, 171, 25, 18, 139, 244, 94,
        123, 224, 205, 32, 241, 174, 36, 120, 199, 206, 199, 202, 216, 154, 2, 10, 247, 101, 138,
        170, 85, 95, 142, 164, 50, 203, 92, 23, 247, 218, 231, 224, 78, 68, 55, 104, 243, 145, 243,
        4, 219, 102, 44, 227,
    ];
    let sig_str =
        "g3Mvj1lET0l5xkZMW0CrGRKL9F574M0g8a4keMfOx8rYmgIK92WKqlVfjqQyy1wX99rn4E5EN2jzkfME22Ys4w";

    pub_key.verify_bytes(op_bytes, sig_str).unwrap();

    let signed = op.into_signed(sig_str.to_string());
    signed.verify_self().unwrap();
}

/*
------------------------------------
OP:
{
  type: 'create',
  signingKey: 'did:key:zDnaesoxZb8mLjf16e4PWsNqLLj9uWM9TQ8nNwxqErDmKXLAN',
  recoveryKey: 'did:key:zDnaesoxZb8mLjf16e4PWsNqLLj9uWM9TQ8nNwxqErDmKXLAN',
  username: 'carla.test',
  service: 'http://localhost:2583',
  prev: null,
  sig: 'VYGxmZs-D5830YdQSNrZpbxVyOPB4nCJtO-x0XElt35AE5wjvJFa2vJu8qjURG6TvEbMvfbekDo_eXEMhdPWdg'
}
ENCODED:
{"0":167,"1":99,"2":115,"3":105,"4":103,"5":120,"6":86,"7":86,"8":89,"9":71,"10":120,"11":109,"12":90,"13":115,"14":45,"15":68,"16":53,"17":56,"18":51,"19":48,"20":89,"21":100,"22":81,"23":83,"24":78,"25":114,"26":90,"27":112,"28":98,"29":120,"30":86,"31":121,"32":79,"33":80,"34":66,"35":52,"36":110,"37":67,"38":74,"39":116,"40":79,"41":45,"42":120,"43":48,"44":88,"45":69,"46":108,"47":116,"48":51,"49":53,"50":65,"51":69,"52":53,"53":119,"54":106,"55":118,"56":74,"57":70,"58":97,"59":50,"60":118,"61":74,"62":117,"63":56,"64":113,"65":106,"66":85,"67":82,"68":71,"69":54,"70":84,"71":118,"72":69,"73":98,"74":77,"75":118,"76":102,"77":98,"78":101,"79":107,"80":68,"81":111,"82":95,"83":101,"84":88,"85":69,"86":77,"87":104,"88":100,"89":80,"90":87,"91":100,"92":103,"93":100,"94":112,"95":114,"96":101,"97":118,"98":246,"99":100,"100":116,"101":121,"102":112,"103":101,"104":102,"105":99,"106":114,"107":101,"108":97,"109":116,"110":101,"111":103,"112":115,"113":101,"114":114,"115":118,"116":105,"117":99,"118":101,"119":117,"120":104,"121":116,"122":116,"123":112,"124":58,"125":47,"126":47,"127":108,"128":111,"129":99,"130":97,"131":108,"132":104,"133":111,"134":115,"135":116,"136":58,"137":50,"138":53,"139":56,"140":51,"141":104,"142":117,"143":115,"144":101,"145":114,"146":110,"147":97,"148":109,"149":101,"150":106,"151":99,"152":97,"153":114,"154":108,"155":97,"156":46,"157":116,"158":101,"159":115,"160":116,"161":106,"162":115,"163":105,"164":103,"165":110,"166":105,"167":110,"168":103,"169":75,"170":101,"171":121,"172":120,"173":57,"174":100,"175":105,"176":100,"177":58,"178":107,"179":101,"180":121,"181":58,"182":122,"183":68,"184":110,"185":97,"186":101,"187":115,"188":111,"189":120,"190":90,"191":98,"192":56,"193":109,"194":76,"195":106,"196":102,"197":49,"198":54,"199":101,"200":52,"201":80,"202":87,"203":115,"204":78,"205":113,"206":76,"207":76,"208":106,"209":57,"210":117,"211":87,"212":77,"213":57,"214":84,"215":81,"216":56,"217":110,"218":78,"219":119,"220":120,"221":113,"222":69,"223":114,"224":68,"225":109,"226":75,"227":88,"228":76,"229":65,"230":78,"231":107,"232":114,"233":101,"234":99,"235":111,"236":118,"237":101,"238":114,"239":121,"240":75,"241":101,"242":121,"243":120,"244":57,"245":100,"246":105,"247":100,"248":58,"249":107,"250":101,"251":121,"252":58,"253":122,"254":68,"255":110,"256":97,"257":101,"258":115,"259":111,"260":120,"261":90,"262":98,"263":56,"264":109,"265":76,"266":106,"267":102,"268":49,"269":54,"270":101,"271":52,"272":80,"273":87,"274":115,"275":78,"276":113,"277":76,"278":76,"279":106,"280":57,"281":117,"282":87,"283":77,"284":57,"285":84,"286":81,"287":56,"288":110,"289":78,"290":119,"291":120,"292":113,"293":69,"294":114,"295":68,"296":109,"297":75,"298":88,"299":76,"300":65,"301":78}
SHA256 base32:
cg2dfxdh5voabmdjzw2abw3sgvtjymknh2bmpvtwot7t2ih4v7za
did:plc:cg2dfxdh5voabmdjzw2abw3s
------------------------------------

*/

#[test]
fn test_debug_did_plc() {
    let op = CreateOp {
        op_type: "create".to_string(),
        signingKey: "did:key:zDnaesoxZb8mLjf16e4PWsNqLLj9uWM9TQ8nNwxqErDmKXLAN".to_string(),
        recoveryKey: "did:key:zDnaesoxZb8mLjf16e4PWsNqLLj9uWM9TQ8nNwxqErDmKXLAN".to_string(),
        username: "carla.test".to_string(),
        service: "http://localhost:2583".to_string(),
        prev: None,
        sig:
            "VYGxmZs-D5830YdQSNrZpbxVyOPB4nCJtO-x0XElt35AE5wjvJFa2vJu8qjURG6TvEbMvfbekDo_eXEMhdPWdg"
                .to_string(),
    };
    op.verify_self().unwrap();
    let block =
        Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, &op).expect("encode DAG-CBOR");
    let op_bytes = block.data();

    let encoded_bytes = vec![
        167, 99, 115, 105, 103, 120, 86, 86, 89, 71, 120, 109, 90, 115, 45, 68, 53, 56, 51, 48, 89,
        100, 81, 83, 78, 114, 90, 112, 98, 120, 86, 121, 79, 80, 66, 52, 110, 67, 74, 116, 79, 45,
        120, 48, 88, 69, 108, 116, 51, 53, 65, 69, 53, 119, 106, 118, 74, 70, 97, 50, 118, 74, 117,
        56, 113, 106, 85, 82, 71, 54, 84, 118, 69, 98, 77, 118, 102, 98, 101, 107, 68, 111, 95,
        101, 88, 69, 77, 104, 100, 80, 87, 100, 103, 100, 112, 114, 101, 118, 246, 100, 116, 121,
        112, 101, 102, 99, 114, 101, 97, 116, 101, 103, 115, 101, 114, 118, 105, 99, 101, 117, 104,
        116, 116, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104, 111, 115, 116, 58, 50, 53, 56, 51,
        104, 117, 115, 101, 114, 110, 97, 109, 101, 106, 99, 97, 114, 108, 97, 46, 116, 101, 115,
        116, 106, 115, 105, 103, 110, 105, 110, 103, 75, 101, 121, 120, 57, 100, 105, 100, 58, 107,
        101, 121, 58, 122, 68, 110, 97, 101, 115, 111, 120, 90, 98, 56, 109, 76, 106, 102, 49, 54,
        101, 52, 80, 87, 115, 78, 113, 76, 76, 106, 57, 117, 87, 77, 57, 84, 81, 56, 110, 78, 119,
        120, 113, 69, 114, 68, 109, 75, 88, 76, 65, 78, 107, 114, 101, 99, 111, 118, 101, 114, 121,
        75, 101, 121, 120, 57, 100, 105, 100, 58, 107, 101, 121, 58, 122, 68, 110, 97, 101, 115,
        111, 120, 90, 98, 56, 109, 76, 106, 102, 49, 54, 101, 52, 80, 87, 115, 78, 113, 76, 76,
        106, 57, 117, 87, 77, 57, 84, 81, 56, 110, 78, 119, 120, 113, 69, 114, 68, 109, 75, 88, 76,
        65, 78,
    ];
    assert_eq!(op_bytes, encoded_bytes);

    let sha256_str = "cg2dfxdh5voabmdjzw2abw3sgvtjymknh2bmpvtwot7t2ih4v7za";
    let _did_plc = "did:plc:cg2dfxdh5voabmdjzw2abw3s";

    let digest_bytes: Vec<u8> = data_encoding::HEXLOWER
        .decode(&sha256::digest(op_bytes).as_bytes())
        .expect("SHA-256 digest is always hex string");
    let digest_b32 = data_encoding::BASE32_NOPAD
        .encode(&digest_bytes)
        .to_ascii_lowercase();
    assert_eq!(digest_b32, sha256_str);
}

#[test]
fn test_did_plc_examples() {
    // https://atproto.com/specs/did-plc
    let op = CreateOp {
        op_type: "create".to_string(),
        signingKey: "did:key:zDnaejYFhgFiVF89LhJ4UipACLKuqo6PteZf8eKDVKeExXUPk".to_string(),
        recoveryKey: "did:key:zDnaeSezF2TgCD71b5DiiFyhHQwKAfsBVqTTHRMvP597Z5Ztn".to_string(),
        username: "alice.example.com".to_string(),
        service: "https://example.com".to_string(),
        prev: None,
        sig:
            "vi6JAl5W4FfyViD5_BKL9p0rbI3MxTWuh0g_egTFAjtf7gwoSfSe1O3qMOEUPX6QH3H0Q9M4y7gOLGblWkEwfQ"
                .to_string(),
    };
    op.verify_self().unwrap();
    assert_eq!(&op.did_plc(), "did:plc:7iza6de2dwap2sbkpav7c6c6");

    // interacting with PDS / PLC server
    let op = CreateOp {
        op_type: "create".to_string(),
        signingKey: "did:key:zDnaekmbFffmpo7LZ4C7bEFjGKPk11N47kKN8j7jtAcGUabw3".to_string(),
        recoveryKey: "did:key:zDnaekmbFffmpo7LZ4C7bEFjGKPk11N47kKN8j7jtAcGUabw3".to_string(),
        username: "voltaire.test".to_string(),
        service: "http://localhost:2583".to_string(),
        prev: None,
        sig:
            "HNfQUg6SMnYKp1l3LtAIsoAblmi33mYiHE9JH1j7w3B-hd8xWpmCUBUoqKfQXmsAs0K1z8Izt19yYk6PqVFgyg"
                .to_string(),
    };
    op.verify_self().unwrap();
    assert_eq!(&op.did_plc(), "did:plc:bmrcg7zrxoiw2kiml3tkw2xv");
}

#[test]
fn test_self_verify() {
    let keypair = KeyPair::new_random();
    let op = CreateOp::new(
        "dummy-handle".to_string(),
        "https://dummy.service".to_string(),
        &keypair,
        None,
    );
    println!("{:?}", op);
    op.verify_self().unwrap();
}

#[test]
fn test_known_key() {
    let keypair = KeyPair::new_random();
    let op = CreateOp::new(
        "dummy-handle".to_string(),
        "https://dummy.service".to_string(),
        &keypair,
        None,
    );
    println!("{:?}", op);
    op.verify_self().unwrap();
}
