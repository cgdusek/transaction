use serde_json::{Map, json};
use rand::Rng;

extern crate ed25519_dalek;

use rand::rngs::OsRng;
use ed25519_dalek::Keypair;
use ed25519_dalek::{Signature, Signer, PublicKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

fn main() {
    println!("Hello, world!");

    let mut keys_values = Map::new();

    let l = 4;
    let m = 4;
    let n = 4;

    let mut rng = rand::thread_rng();
    println!("Integer from range n: {}", rng.gen_range(0, m));
    println!("Integer from range m: {}", rng.gen_range(0, n));

    for _i in 0..l {
        let o :u64 = rng.gen_range(0, m);
        let p :u64 = rng.gen_range(0, n);
        keys_values.insert(o.to_string(), json!(p));
    }

    println!("{}", serde_json::to_string_pretty(&keys_values).unwrap());

    let mut csprng = OsRng{};
    let keypair: Keypair = Keypair::generate(&mut OsRng);
    let public_key: PublicKey = keypair.public;
    let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = public_key.to_bytes();

    let message: &[u8] = b"This is a test of the tsunami alert system.";
    let signature: Signature = keypair.sign(message);

    println!("\n\nWrite public key to json");
    let demo_json = json!(public_key_bytes);
    println!("{}", demo_json);

}
