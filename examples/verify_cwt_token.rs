use api::auth::verify_token_direct;
use ed25519_dalek::VerifyingKey;
use hex;

fn main() {
    // Token from create_token
    let token = "hEOhASegWD6mAmExBBpyfYmvYWt4JDAxOWE4M2ViLWY4YWQtN2ZlMS1iMjE1LWJkNGNmZWQ4OTE3OWF0AGFtGIBhcRlOIFhAeludKLKEhKaqkVso5pOn7oNB3wfJps1Sw4pb6Jls5Dh3R7/mAmPlFShUmd1D8OebGEDJMl1nkKHGITlUXASuAw==";

    // Public key
    let public_key_hex = "74dc80faf54b0c35ded8a19223b14e885f5fed4754b8c709a31e63d92290781c";
    let public_key_bytes = hex::decode(public_key_hex).expect("Invalid hex");
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes.try_into().unwrap()).expect("Invalid public key");

    // Verify token
    println!("=== Verifying CWT Token ===\n");
    match verify_token_direct(token, &verifying_key) {
        Ok(claims) => {
            println!("✅ Token verification successful!");
            println!("\nExtracted Claims:");
            println!("  User ID: {}", claims.user_id());
            println!("  Key ID: {}", claims.key_id());
            println!("  Tier: {:?}", claims.tier().unwrap());
            println!("  Max Tokens: {}", claims.max_tokens());
            println!("  Monthly Quota: {}", claims.monthly_quota());
            println!("  Expiration: {}", claims.exp().unwrap());
        }
        Err(e) => {
            println!("❌ Token verification failed: {}", e);
            std::process::exit(1);
        }
    }
}
