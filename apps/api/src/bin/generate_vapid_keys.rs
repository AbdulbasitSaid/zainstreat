//! Prints a fresh VAPID key pair for admin web push (Phase 27 Decision 16),
//! in the env format the API reads. One pair per environment; keep the
//! private key in a secret, never in the repo.

use web_push_native::jwt_simple::algorithms::ES256KeyPair;

fn main() {
    let key_pair = ES256KeyPair::generate();
    println!("VAPID_PUBLIC_KEY={}", api::push::encode_public_key(&key_pair));
    println!("VAPID_PRIVATE_KEY={}", api::push::encode_private_key(&key_pair));
}
