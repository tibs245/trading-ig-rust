# Security policy

## Reporting a vulnerability — Signaler une vulnérabilité

### 🇫🇷 Français

Si vous découvrez une vulnérabilité dans `trading-ig`, **merci de ne
pas l'ouvrir publiquement**. Contactez directement le mainteneur :

- **Email** : <thibault.barske@kolombo.xyz>
- **Avis de sécurité GitHub** :
  <https://github.com/tibs245/trading-ig-rust/security/advisories/new>

Merci d'inclure dans votre rapport :

- Les features Cargo nécessaires pour reproduire (`default`, `stream`,
  `encryption`, `polars`, `live`, …).
- La version Rust minimale concernée.
- Un PoC ou un scénario d'exploitation, si possible.
- Si le problème est exploitable contre un compte IG démo ou
  uniquement via une configuration personnalisée.

Vous recevrez une réponse sous 72h ouvrées.

### 🇬🇧 English

If you discover a vulnerability in `trading-ig`, **please do not open
it publicly**. Contact the maintainer directly:

- **Email**: <thibault.barske@kolombo.xyz>
- **GitHub Security Advisory**:
  <https://github.com/tibs245/trading-ig-rust/security/advisories/new>

Please include in your report:

- The cargo features required to reproduce (`default`, `stream`,
  `encryption`, `polars`, `live`, …).
- The minimum Rust toolchain affected.
- A PoC or exploitation scenario if available.
- Whether the issue is exploitable against an IG demo account or
  only through a custom configuration.

You will receive a response within 72 business hours.

## Recommendation for funded accounts — Recommandation pour comptes funded

> 🇫🇷 **Pour tout compte avec de l'argent réel ou simulé important
> (live, démo financée), nous recommandons fortement d'activer la
> feature `encryption` et d'utiliser `session().login_with_encryption()`
> au lieu de `session().login()`.**
>
> Le mot de passe est alors chiffré côté client (RSA PKCS#1 v1.5 avec la
> clé publique d'IG) avant transmission, donc :
>
> - Il ne traverse aucun proxy intermédiaire en clair.
> - Il n'apparaît jamais en clair dans des logs côté serveur ou
>   middleware.
> - Il reste protégé même en cas de compromission d'une autorité de
>   certification ou de man-in-the-middle TLS.

> 🇬🇧 **For any account holding real money or significant simulated
> funds (live, funded demo), we strongly recommend enabling the
> `encryption` feature and using `session().login_with_encryption()`
> instead of `session().login()`.**
>
> The password is then encrypted client-side (RSA PKCS#1 v1.5 with IG's
> public key) before being sent over the wire, so:
>
> - It doesn't traverse any intermediate proxy in plaintext.
> - It never appears in plaintext in server-side or middleware logs.
> - It remains protected even in the event of a compromised certificate
>   authority or TLS man-in-the-middle.

```toml
[dependencies]
trading-ig = { version = "0.1", features = ["encryption"] }
```

```rust
client.session().login_with_encryption().await?;
//             ^^^^^^^^^^^^^^^^^^^^^^^^
// instead of: .login()
```

## Known advisory — Avis connu

### RUSTSEC-2023-0071 (`rsa` crate — Marvin Attack)

- **Status / statut**: acknowledged, **not applicable to this crate**.
- **Affected dependency / dépendance impactée**: `rsa = "0.9"`, only
  pulled in by the optional `encryption` feature.
- **Why it does not apply / pourquoi ça ne s'applique pas**: the attack
  targets PKCS#1 v1.5 *decryption* through timing side-channels. Our
  only use of the `rsa` crate is `RsaPublicKey::encrypt(...)` in
  [`session::encryption::encrypt_password`](src/session/encryption.rs).
  The crate never decrypts ciphertexts and does not hold a private key.
  The Marvin attack requires a victim that decrypts, which is not the
  case here.
- **Tracking**: an `ignore` entry exists in `deny.toml` and in the CI
  workflow's `cargo audit` invocation. The entry will be removed once
  the upstream `rsa` crate releases a constant-time implementation
  (<https://github.com/RustCrypto/RSA/issues/19>).

## Defensive practices in this crate — Pratiques défensives

- Credentials and tokens are **never written to logs**. The `tracing`
  spans intentionally redact `Authorization`, `CST`, and
  `X-SECURITY-TOKEN` headers.
- All HTTP traffic uses TLS (`rustls` by default; `native-tls`
  optional). Plaintext HTTP is not supported.
- The published crate package excludes test fixtures, internal
  knowledge files, CI configs, and git hooks (see `Cargo.toml`'s
  `exclude` list). No credentials, secrets, or sample `.env` files
  ship to crates.io.
- The `live` and `live-trading` cargo features gate the manual live
  test suite — they are never compiled by `cargo test --all-features`
  in CI, and they require explicit environment variables to run.
- `cargo audit` runs on every push (`.github/workflows/ci.yml`) and
  weekly (`.github/workflows/audit.yml`), with `cargo deny` covering
  licenses, banned crates, and source provenance.
