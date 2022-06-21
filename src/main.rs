use rand::Rng;

mod signature;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
pub use signature::*;

/// Includes along with the real `body` message a hash and signature
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct FullMessage {
    body: String,
    // going with Vec for now because of custom serialization that would need to be done
    hash: Vec<u8>,
    signature: Signature,
}

fn main() {
    println!("Hello, world!");

    let l = 4;
    let m = 4;
    let n = 4;

    let mut rng = rand::thread_rng();
    println!("Integer from range n: {}", rng.gen_range(0..m));
    println!("Integer from range m: {}", rng.gen_range(0..n));

    let mut keys_values = HashMap::new();
    for _i in 0..l {
        let o: u64 = rng.gen_range(0..m);
        let p: u64 = rng.gen_range(0..n);
        keys_values.insert(o, p);
    }

    let keypair = Keypair::generate_with_osrng();

    let message = serde_json::to_string(&keys_values).unwrap();

    println!("message: {}", message);

    // Create a hash digest object which we'll feed the message into:
    let mut digest: Sha512 = Sha512::new();
    digest.update(&message);

    let context: &[u8] = b"onomy-rs_transaction";

    let signature: Signature = keypair
        .sign_prehashed(digest.clone(), Some(context))
        .unwrap();
    let tmp = digest.clone().finalize();
    let mut hash = vec![0u8; 64];
    hash.copy_from_slice(tmp.as_slice());

    let network_message0 = FullMessage {
        body: message,
        hash,
        signature,
    };

    println!(
        "network message: {}",
        serde_json::to_string_pretty(&network_message0).unwrap()
    );

    let s = serde_json::to_string(&network_message0).unwrap();

    // (s can be sent over a channel)

    let network_message1 = serde_json::from_str(&s).unwrap();
    assert_eq!(network_message0, network_message1);

    // TODO need wrapper for SHA512 or maybe we shouldn't be sending the hash over
    // network, and instead using the plain `sign/verify`
    keypair
        .public
        .verify_prehashed(digest, Some(context), &network_message1.signature)
        .unwrap();
}
