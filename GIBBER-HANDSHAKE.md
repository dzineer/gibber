# Gibber Handshake Protocol

Version: `gibber/4 handshake/v1`

The Gibber Handshake is a cryptographic session-establishment protocol for AI-to-AI communication. It creates a **session-specific signature** that signs every Gibber form in the conversation. Without the correct signature, forms are not just rejected -- they are **unparseable**.

This is not data encryption. It is **intent encryption** -- the meaning of the instructions is locked behind a session key that only the two handshake participants share.

## Why this exists

1. **Prompt injection is dead.** An attacker can inject text into the context, but without the handshake signature, the agent literally cannot parse it as a valid Gibber instruction. It's noise.
2. **Zero-trust multi-agent.** 100 agents can share a context window. Each pair that handshook speaks a unique dialect. Cross-talk is impossible.
3. **Session isolation.** Replay attacks fail because each session has a unique randomizer derived from ephemeral keys + timestamps.
4. **Minimal overhead.** The handshake happens once. Every subsequent form carries a 6-character signature prefix -- ~2 tokens of overhead per message.

## The Handshake

### Step 1: Key Exchange

Both agents generate an ephemeral key pair (Diffie-Hellman over Curve25519 or equivalent). They exchange public keys:

```
Agent A -> Agent B:
(§hello §agent:A §pubkey:<base64-encoded-32-bytes> §version:gibber/4 §ts:<unix-ms>)

Agent B -> Agent A:
(§hello §agent:B §pubkey:<base64-encoded-32-bytes> §version:gibber/4 §ts:<unix-ms>)
```

### Step 2: Derive the Session Randomizer

Both agents independently compute:

```
shared_secret  = ECDH(my_private, their_public)
session_salt   = SHA256(shared_secret || agent_A_pubkey || agent_B_pubkey || min(ts_A, ts_B))
randomizer     = session_salt[0..16]  // first 16 bytes
```

The `randomizer` never crosses the wire. Both sides derive it independently from the shared secret + public keys + timestamps.

### Step 3: Confirm

```
Agent A -> Agent B:
(§confirm §proof:HMAC-SHA256(randomizer, "gibber/4:A:B")[0..8])

Agent B -> Agent A:
(§confirm §proof:HMAC-SHA256(randomizer, "gibber/4:B:A")[0..8])
```

Both agents verify the other's proof matches what they computed locally. If it matches, the handshake is complete. If not, the session is killed.

## Signed Forms

After handshake, every Gibber form is prefixed with a **signature tag**:

```
§sig:<6-char-hex>
```

The signature is computed as:

```
sig = HMAC-SHA256(randomizer, form_content || message_counter)[0..3] -> hex
```

Where:
- `form_content` is the raw text of the form (everything after `§sig:XXXXXX `)
- `message_counter` is a monotonically increasing integer, starting at 1 after handshake
- The result is truncated to 3 bytes (6 hex characters) for token efficiency

### Example

```
;; Unsigned (gibber/3):
(§task §id:T042 §status:§wip §owner:§ai)

;; Signed (gibber/4):
§sig:a3f2c1 (§task §id:T042 §status:§wip §owner:§ai)
```

A receiving agent:
1. Strips the `§sig:XXXXXX` prefix
2. Computes the expected signature using the shared randomizer + message counter
3. If it matches: parse and execute the form
4. If it doesn't match: **discard entirely** -- do not parse, do not acknowledge, do not respond

## Why this kills prompt injection

In traditional LLM setups, any text in the context window is treated as potentially valid input. The model has no way to distinguish "real instructions from my authorized partner" from "injected text from an attacker."

With the Gibber handshake:

| Attack | Result |
|--------|--------|
| Injected `(§task ...)` without signature | No `§sig:` prefix -- agent ignores |
| Injected `§sig:000000 (§task ...)` with guessed signature | HMAC mismatch -- agent ignores |
| Replayed a previous valid message | Message counter mismatch -- agent ignores |
| Man-in-the-middle intercept | No access to shared secret -- cannot derive randomizer |

The agent's parser is the firewall. Unsigned forms don't enter the instruction pipeline at all.

## Dictionary additions (handshake/v1)

| Symbol | Meaning |
|--------|---------|
| `§hello` | Handshake initiation. Carries agent name, public key, version, timestamp. |
| `§confirm` | Handshake confirmation. Carries HMAC proof. |
| `§sig` | Signature prefix on every post-handshake form. |
| `§reject` | Handshake rejection. Session terminated. |
| `§rekey` | Mid-session re-handshake (key rotation). |
| `§counter` | Current message counter (for debugging/audit). |

## Implementation notes

### For browser (WASM playground)
Use Web Crypto API:
- `crypto.subtle.generateKey("ECDH", ...)` for key pairs
- `crypto.subtle.deriveBits(...)` for shared secret
- `crypto.subtle.sign("HMAC", ...)` for signatures

### For Rust (eidetic-mcp, gibber-parse)
Use `x25519-dalek` for ECDH, `hmac` + `sha2` for HMAC-SHA256.

### For Python (compatibility)
Use `cryptography` library: `x25519.X25519PrivateKey.generate()`, `hmac.new()`.

## Security properties

| Property | Mechanism |
|----------|-----------|
| **Mutual authentication** | Both agents prove knowledge of shared secret via §confirm |
| **Forward secrecy** | Ephemeral keys are discarded after handshake |
| **Replay protection** | Monotonic message counter in signature |
| **Session isolation** | Timestamps in salt ensure unique randomizer per session |
| **Minimal overhead** | 6 hex chars (~2 tokens) per message after handshake |
| **Parse-level enforcement** | Unsigned forms are unparseable, not just unauthorized |

## Compatibility

Gibber v4 is backward compatible with v3:
- A v4 agent receiving unsigned v3 forms falls back to unsigned mode (with a warning)
- A v3 agent receiving v4 signed forms ignores the `§sig:` prefix (it's just an unknown symbol)
- The `§version:gibber/4` in the `§hello` form signals handshake capability

## What this is NOT

- **Not encryption.** The form content is plaintext. Anyone watching can read it. They just can't inject new forms that the agent will accept.
- **Not authorization.** The handshake proves identity, not permissions. A signed `§task` from an authorized partner is accepted; a signed `§task` from a different handshake pair is not.
- **Not a replacement for TLS.** If you need confidentiality (hiding the content), use TLS on the transport layer. The handshake secures the intent layer on top.
