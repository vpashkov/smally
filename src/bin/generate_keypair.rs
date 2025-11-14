use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn main() {
    println!("Generating Ed25519 keypair for token signing...\n");

    // Generate a new keypair
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    // Convert to hex
    let private_key_hex = hex::encode(signing_key.to_bytes());
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    println!("=== Ed25519 Keypair Generated ===\n");
    println!("Private Key (keep secret!):");
    println!("{}\n", private_key_hex);
    println!("Public Key (share with workers):");
    println!("{}\n", public_key_hex);

    println!("=== .env Configuration ===\n");
    println!("# Add these to your .env file:");
    println!("TOKEN_PRIVATE_KEY={}", private_key_hex);
    println!("TOKEN_PUBLIC_KEY={}", public_key_hex);
    println!("\n=== Security Notice ===");
    println!("- Keep TOKEN_PRIVATE_KEY secret (only on auth server)");
    println!("- TOKEN_PUBLIC_KEY can be shared (on all worker nodes)");
    println!("- Never commit private key to version control");
}
