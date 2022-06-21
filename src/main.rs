use rand::Rng;
use serde_json::{json, Map};

mod signature;
pub use signature::*;
extern crate ed25519_dalek;

use rand::rngs::OsRng;

fn main() {
    println!("Hello, world!");

    let mut keys_values = Map::new();
    let mut body = Map::new();
    let mut tx = Map::new();

    let l = 4;
    let m = 4;
    let n = 4;

    let nonce: u64 = 1;

    let mut rng = rand::thread_rng();
    println!("Integer from range n: {}", rng.gen_range(0..m));
    println!("Integer from range m: {}", rng.gen_range(0..n));

    for _i in 0..l {
        let o: u64 = rng.gen_range(0..m);
        let p: u64 = rng.gen_range(0..n);
        keys_values.insert(o.to_string(), json!(p));
    }

    println!("{}", serde_json::to_string_pretty(&keys_values).unwrap());

    let csprng = OsRng {};
    /*let keypair = ed25519_dalek::Keypair::generate(&mut OsRng);
        let public_key: ed25519_dalek::PublicKey = keypair.public;
        let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = public_key.to_bytes();
        let public_key_json = json!(public_key_bytes);

        body.insert("public_key".to_string(), public_key_json);
        body.insert("keys".to_string(), json!(keys_values));
        body.insert("nonce".to_string(), json!(nonce));

        println!("{}", serde_json::to_string_pretty(&body).unwrap());

        let message = serde_json::to_string(&body).unwrap();

        // Create a hash digest object which we'll feed the message into:
        let mut prehashed: Sha512 = Sha512::new();

        prehashed.update(message);

        let context: &[u8] = b"onomy-rs_transaction";

        let signature: ed25519_dalek::Signature = keypair
            .sign_prehashed(prehashed.clone(), Some(context))
            .unwrap();

        let signature = crate::Signature {
            signature: signature,
        };

        dbg!(&signature);
        let s = serde_json::to_string(&signature).unwrap();
        dbg!(&s);
        let sig: Signature = serde_json::from_str(&s).unwrap();
        dbg!(sig);
    */
    /*let signature_bytes: [u8; SIGNATURE_LENGTH] = signature.to_bytes();
    let prehashed_bytes = prehashed.finalize();



    // or use `serde_json::to_vec`
    dbg!(serde_json::to_vec(&signature).unwrap());
    tx.insert("hash".to_string(), json!(str::from_utf8(&prehashed_bytes).unwrap()));
    tx.insert("body".to_string(), json!(body));
    tx.insert("signature".to_string(), json!(str::from_utf8(&signature_bytes).unwrap()));

    println!("{}", serde_json::to_string_pretty(&tx).unwrap());*/
}
