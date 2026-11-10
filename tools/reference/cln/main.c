/* Core Lightning's answers for the cross-check's `lightning` area
   (tools/crosscheck/area_lightning.py): a request a line on standard
   input, an answer a line on standard output. Faraday's; the functions it
   calls are Core Lightning's, and README.md says which are compiled from
   its files and which are written out here from its common/hsm_secret.c.

     version                       libsodium's version string
     hkdf SECRET SALT              Core Lightning's HKDF-SHA256 (ccan) with
                                   the info "nodeid" and the salt as a
                                   4-byte unsigned number in this machine's
                                   byte order, as hsmd passes it: 32 bytes
     open FILE PASSPHRASE          get_encryption_key then decrypt_hsm_secret
     seal SECRET PASSPHRASE        get_encryption_key then
                                   encrypt_legacy_hsm_secret, a new header
                                   each time
     mnemonic FILE PASSPHRASE VALID SEED
                                   detect_hsm_secret_type and
                                   extract_mnemonic_secret of a file longer
                                   than 32 bytes and not 73: the first 32
                                   bytes of the BIP-39 seed. PASSPHRASE is
                                   "none" for no passphrase (NULL). VALID
                                   (1 or 0) is validate_mnemonic's answer
                                   and SEED (64 bytes) is
                                   bip39_mnemonic_to_seed's, both libwally's
                                   and not compiled here: the area gives
                                   python-mnemonic's.

   Arguments are hex; "-" is the empty string. An answer is "ok HEX" or
   "err". */
#include "config.h"
#include <ccan/crypto/hkdf_sha256/hkdf_sha256.h>
#include <ccan/crypto/sha256/sha256.h>
#include <sodium.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct secret { unsigned char data[32]; };

#define HS_HEADER_LEN crypto_secretstream_xchacha20poly1305_HEADERBYTES
#define HS_CIPHERTEXT_LEN \
	(sizeof(struct secret) + crypto_secretstream_xchacha20poly1305_ABYTES)

/* From common/hsm_secret.c, get_encryption_key: the same salt, bounds and
   libsodium call; the tal allocation and mlock of the secret are left out. */
static int get_encryption_key(struct secret *secret, const char *passphrase)
{
	const unsigned char salt[16] = "c-lightning\0\0\0\0\0";

	if (strlen(passphrase) < crypto_pwhash_argon2id_PASSWD_MIN)
		return 0;
	else if (strlen(passphrase) > crypto_pwhash_argon2id_PASSWD_MAX)
		return 0;
	if (crypto_pwhash(secret->data, sizeof(secret->data), passphrase, strlen(passphrase), salt,
			  crypto_pwhash_argon2id_OPSLIMIT_MODERATE,
			  crypto_pwhash_argon2id_MEMLIMIT_MODERATE,
			  crypto_pwhash_ALG_ARGON2ID13) != 0)
		return 0;
	return 1;
}

/* From common/hsm_secret.c, decrypt_hsm_secret, unchanged. */
static int decrypt_hsm_secret(const struct secret *encryption_key,
			      const unsigned char *cipher,
			      struct secret *output)
{
	crypto_secretstream_xchacha20poly1305_state crypto_state;

	if (crypto_secretstream_xchacha20poly1305_init_pull(&crypto_state, cipher,
							    encryption_key->data) != 0)
		return 0;
	if (crypto_secretstream_xchacha20poly1305_pull(&crypto_state, output->data,
						       NULL, 0,
						       cipher + HS_HEADER_LEN,
						       HS_CIPHERTEXT_LEN,
						       NULL, 0) != 0)
		return 0;
	return 1;
}

/* From common/hsm_secret.c, encrypt_legacy_hsm_secret, unchanged. */
static int encrypt_legacy_hsm_secret(const struct secret *encryption_key,
				     const struct secret *hsm_secret,
				     unsigned char *output)
{
	crypto_secretstream_xchacha20poly1305_state crypto_state;

	if (crypto_secretstream_xchacha20poly1305_init_push(&crypto_state, output,
							    encryption_key->data) != 0)
		return 0;
	if (crypto_secretstream_xchacha20poly1305_push(&crypto_state,
						       output + HS_HEADER_LEN,
						       NULL, hsm_secret->data,
						       sizeof(hsm_secret->data),
						       NULL, 0, 0))
		return 0;
	return 1;
}

#define HSM_SECRET_PLAIN_SIZE 32
#define ENCRYPTED_HSM_SECRET_LEN (HS_HEADER_LEN + HS_CIPHERTEXT_LEN)
#define PASSPHRASE_HASH_LEN 32

/* The checks of common/hsm_secret.c's extract_mnemonic_secret, in its
   order, over the file's bytes: the hash of the seed is all zero for "no
   passphrase" (detect_hsm_secret_type); a passphrase is required when it
   is not and refused when it is; a given passphrase's seed must hash to
   the stored value (derive_seed_hash, then sha256_eq); the words must
   validate. `passphrase` is NULL when there is none. */
static int extract_mnemonic_secret(const unsigned char *file, size_t len,
				   const char *passphrase, int valid,
				   const unsigned char seed[64],
				   unsigned char out[32])
{
	int none = 1;
	struct sha256 stored, computed;

	if (len <= HSM_SECRET_PLAIN_SIZE || len == ENCRYPTED_HSM_SECRET_LEN)
		return 0;
	for (size_t i = 0; i < PASSPHRASE_HASH_LEN; i++)
		if (file[i])
			none = 0;
	if (!none) {
		if (!passphrase)
			return 0;
		memcpy(&stored, file, sizeof(stored));
		sha256(&computed, seed, 64);
		if (memcmp(&stored, &computed, sizeof(stored)) != 0)
			return 0;
	} else if (passphrase) {
		return 0;
	}
	if (!valid)
		return 0;
	memcpy(out, seed, 32);
	return 1;
}

static size_t unhex(const char *s, unsigned char *out, size_t max)
{
	size_t n = strcmp(s, "-") == 0 ? 0 : strlen(s);
	size_t len = 0;
	if (n % 2 || n / 2 > max)
		return (size_t)-1;
	for (size_t i = 0; i < n; i += 2) {
		unsigned int b;
		if (sscanf(s + i, "%2x", &b) != 1)
			return (size_t)-1;
		out[len++] = (unsigned char)b;
	}
	return len;
}

static void say(const unsigned char *b, size_t n)
{
	printf("ok ");
	for (size_t i = 0; i < n; i++)
		printf("%02x", b[i]);
	printf("\n");
}

int main(void)
{
	char line[2048];

	if (sodium_init() < 0)
		return 2;
	setvbuf(stdout, NULL, _IOLBF, 0);
	while (fgets(line, sizeof(line), stdin)) {
		char op[16] = "", a[1024] = "", b[1024] = "", v[8] = "", d[160] = "";
		unsigned char x[512], y[256], pass[128];
		struct secret key, out;
		if (sscanf(line, "%15s %1023s %1023s %7s %159s", op, a, b, v, d) < 1) {
			printf("err\n");
			continue;
		}
		if (strcmp(op, "version") == 0) {
			printf("ok %s\n", sodium_version_string());
		} else if (strcmp(op, "hkdf") == 0) {
			size_t n = unhex(a, x, sizeof(x));
			uint32_t salt = (uint32_t)strtoul(b, NULL, 10);
			if (n != 32) { printf("err\n"); continue; }
			hkdf_sha256(out.data, sizeof(out.data), &salt, sizeof(salt),
				    x, 32, "nodeid", 6);
			say(out.data, 32);
		} else if (strcmp(op, "open") == 0 || strcmp(op, "seal") == 0) {
			size_t n = unhex(a, x, sizeof(x));
			size_t p = unhex(b, pass, sizeof(pass) - 1);
			if (n == (size_t)-1 || p == (size_t)-1) { printf("err\n"); continue; }
			pass[p] = 0;
			if (!get_encryption_key(&key, (const char *)pass)) { printf("err\n"); continue; }
			if (op[0] == 'o') {
				if (n != HS_HEADER_LEN + HS_CIPHERTEXT_LEN
				    || !decrypt_hsm_secret(&key, x, &out)) { printf("err\n"); continue; }
				say(out.data, 32);
			} else {
				if (n != 32) { printf("err\n"); continue; }
				memcpy(out.data, x, 32);
				if (!encrypt_legacy_hsm_secret(&key, &out, y)) { printf("err\n"); continue; }
				say(y, HS_HEADER_LEN + HS_CIPHERTEXT_LEN);
			}
		} else if (strcmp(op, "mnemonic") == 0) {
			unsigned char seed[64];
			size_t n = unhex(a, x, sizeof(x));
			int none = strcmp(b, "none") == 0;
			size_t p = none ? 0 : unhex(b, pass, sizeof(pass) - 1);
			if (n == (size_t)-1 || p == (size_t)-1
			    || unhex(d, seed, sizeof(seed)) != sizeof(seed)) { printf("err\n"); continue; }
			pass[p] = 0;
			if (!extract_mnemonic_secret(x, n, none ? NULL : (const char *)pass,
						     v[0] == '1', seed, out.data)) { printf("err\n"); continue; }
			say(out.data, 32);
		} else {
			printf("err\n");
		}
	}
	return 0;
}
