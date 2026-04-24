#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Env, String, Symbol,
    contracterror, symbol_short,
    log,
};

// ─────────────────────────────────────────────
//  Storage keys
// ─────────────────────────────────────────────
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

// ─────────────────────────────────────────────
//  Data types stored on-chain
// ─────────────────────────────────────────────

/// KYC tier: how thoroughly the user was verified.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum KycLevel {
    /// Basic identity check (name + DOB)
    Basic,
    /// Full document verification (passport / national ID)
    Standard,
    /// Enhanced due-diligence (AML screening included)
    Enhanced,
}

/// The on-chain KYC credential issued to a wallet address.
#[contracttype]
#[derive(Clone, Debug)]
pub struct KycCredential {
    /// Wallet that owns this credential
    pub owner: Address,
    /// Verification level granted
    pub level: KycLevel,
    /// UNIX timestamp when credential was issued
    pub issued_at: u64,
    /// UNIX timestamp when credential expires (0 = never)
    pub expires_at: u64,
    /// Opaque reference to the off-chain KYC report (e.g. a hash / UUID)
    pub reference: String,
    /// Whether this credential is currently active
    pub active: bool,
}

// ─────────────────────────────────────────────
//  Contract errors
// ─────────────────────────────────────────────
#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum KycError {
    /// Caller is not the contract admin
    Unauthorized = 1,
    /// A credential already exists for this address
    AlreadyIssued = 2,
    /// No credential found for this address
    NotFound = 3,
    /// Credential has expired
    Expired = 4,
    /// Credential has been revoked
    Revoked = 5,
    /// Contract has already been initialised
    AlreadyInitialised = 6,
}

// ─────────────────────────────────────────────
//  Helper — storage key per address
// ─────────────────────────────────────────────
fn cred_key(owner: &Address) -> Address {
    owner.clone()
}

// ─────────────────────────────────────────────
//  Contract
// ─────────────────────────────────────────────
#[contract]
pub struct KycCredentialContract;

#[contractimpl]
impl KycCredentialContract {

    // ── Initialisation ──────────────────────────────────────────────────

    /// Deploy the contract and set the admin address.
    /// Can only be called once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), KycError> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(KycError::AlreadyInitialised);
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN_KEY, &admin);
        log!(&env, "KYC contract initialised. Admin: {}", admin);
        Ok(())
    }

    // ── Admin helpers ───────────────────────────────────────────────────

    /// Return the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN_KEY).unwrap()
    }

    /// Transfer admin role to a new address.
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), KycError> {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();
        env.storage().instance().set(&ADMIN_KEY, &new_admin);
        log!(&env, "Admin transferred to: {}", new_admin);
        Ok(())
    }

    // ── Issuing credentials ─────────────────────────────────────────────

    /// Issue a KYC credential to `owner`.
    ///
    /// * `expires_at` — pass `0` for a non-expiring credential.
    /// * `reference`  — hash / UUID pointing to the off-chain report.
    pub fn issue(
        env: Env,
        owner: Address,
        level: KycLevel,
        expires_at: u64,
        reference: String,
    ) -> Result<KycCredential, KycError> {
        // Only admin may issue
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        // Prevent duplicate credentials
        if env.storage().persistent().has(&cred_key(&owner)) {
            return Err(KycError::AlreadyIssued);
        }

        let credential = KycCredential {
            owner: owner.clone(),
            level,
            issued_at: env.ledger().timestamp(),
            expires_at,
            reference,
            active: true,
        };

        env.storage()
            .persistent()
            .set(&cred_key(&owner), &credential);

        log!(&env, "Credential issued to: {}", owner);
        Ok(credential)
    }

    // ── Querying credentials ────────────────────────────────────────────

    /// Fetch the raw credential record (does NOT validate expiry / revocation).
    pub fn get_credential(env: Env, owner: Address) -> Result<KycCredential, KycError> {
        env.storage()
            .persistent()
            .get(&cred_key(&owner))
            .ok_or(KycError::NotFound)
    }

    /// Returns `true` only when the credential exists, is active, and has not expired.
    pub fn is_verified(env: Env, owner: Address) -> bool {
        let cred: Option<KycCredential> = env
            .storage()
            .persistent()
            .get(&cred_key(&owner));

        match cred {
            None => false,
            Some(c) => {
                if !c.active {
                    return false; // revoked
                }
                if c.expires_at != 0 && env.ledger().timestamp() > c.expires_at {
                    return false; // expired
                }
                true
            }
        }
    }

    /// Returns `true` only when the credential meets *at least* the required level.
    pub fn meets_level(env: Env, owner: Address, required: KycLevel) -> bool {
        let cred: Option<KycCredential> = env
            .storage()
            .persistent()
            .get(&cred_key(&owner));

        match cred {
            None => false,
            Some(c) => {
                if !c.active {
                    return false;
                }
                if c.expires_at != 0 && env.ledger().timestamp() > c.expires_at {
                    return false;
                }
                // Level ordering: Basic < Standard < Enhanced
                let level_rank = |l: &KycLevel| match l {
                    KycLevel::Basic    => 0u32,
                    KycLevel::Standard => 1u32,
                    KycLevel::Enhanced => 2u32,
                };
                level_rank(&c.level) >= level_rank(&required)
            }
        }
    }

    // ── Updating / revoking ─────────────────────────────────────────────

    /// Upgrade an existing credential to a higher level or update its expiry.
    pub fn update(
        env: Env,
        owner: Address,
        new_level: KycLevel,
        new_expires_at: u64,
        new_reference: String,
    ) -> Result<KycCredential, KycError> {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        let mut cred: KycCredential = env
            .storage()
            .persistent()
            .get(&cred_key(&owner))
            .ok_or(KycError::NotFound)?;

        cred.level      = new_level;
        cred.expires_at = new_expires_at;
        cred.reference  = new_reference;
        // re-activate in case it had been revoked (admin decision)
        cred.active     = true;

        env.storage()
            .persistent()
            .set(&cred_key(&owner), &cred);

        log!(&env, "Credential updated for: {}", owner);
        Ok(cred)
    }

    /// Revoke a credential — marks it inactive without deleting the audit trail.
    pub fn revoke(env: Env, owner: Address) -> Result<(), KycError> {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        let mut cred: KycCredential = env
            .storage()
            .persistent()
            .get(&cred_key(&owner))
            .ok_or(KycError::NotFound)?;

        cred.active = false;
        env.storage()
            .persistent()
            .set(&cred_key(&owner), &cred);

        log!(&env, "Credential revoked for: {}", owner);
        Ok(())
    }

    /// Permanently delete a credential record (GDPR right-to-erasure).
    pub fn delete(env: Env, owner: Address) -> Result<(), KycError> {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&cred_key(&owner)) {
            return Err(KycError::NotFound);
        }

        env.storage().persistent().remove(&cred_key(&owner));
        log!(&env, "Credential deleted for: {}", owner);
        Ok(())
    }
}

// ─────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env};

    fn setup() -> (Env, KycCredentialContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, KycCredentialContract);
        let client      = KycCredentialContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user  = Address::generate(&env);

        client.initialize(&admin);
        (env, client, admin, user)
    }

    #[test]
    fn test_issue_and_verify() {
        let (env, client, _admin, user) = setup();
        let reference = soroban_sdk::String::from_str(&env, "report-abc-123");

        client.issue(&user, &KycLevel::Standard, &0, &reference);
        assert!(client.is_verified(&user));
    }

    #[test]
    fn test_revoke() {
        let (env, client, _admin, user) = setup();
        let reference = soroban_sdk::String::from_str(&env, "report-xyz-456");

        client.issue(&user, &KycLevel::Basic, &0, &reference);
        assert!(client.is_verified(&user));

        client.revoke(&user);
        assert!(!client.is_verified(&user));
    }

    #[test]
    fn test_expiry() {
        let (env, client, _admin, user) = setup();
        let reference = soroban_sdk::String::from_str(&env, "report-exp-789");

        // Expires at ledger timestamp 1000
        client.issue(&user, &KycLevel::Basic, &1000, &reference);
        assert!(client.is_verified(&user));

        // Advance ledger past expiry
        env.ledger().with_mut(|l| l.timestamp = 1001);
        assert!(!client.is_verified(&user));
    }

    #[test]
    fn test_meets_level() {
        let (env, client, _admin, user) = setup();
        let reference = soroban_sdk::String::from_str(&env, "report-level-999");

        client.issue(&user, &KycLevel::Standard, &0, &reference);

        assert!(client.meets_level(&user, &KycLevel::Basic));
        assert!(client.meets_level(&user, &KycLevel::Standard));
        assert!(!client.meets_level(&user, &KycLevel::Enhanced));
    }

    #[test]
    fn test_duplicate_issue_fails() {
        let (env, client, _admin, user) = setup();
        let r1 = soroban_sdk::String::from_str(&env, "ref-1");
        let r2 = soroban_sdk::String::from_str(&env, "ref-2");

        client.issue(&user, &KycLevel::Basic, &0, &r1);
        let result = client.try_issue(&user, &KycLevel::Standard, &0, &r2);
        assert!(result.is_err());
    }

    #[test]
    fn test_double_init_fails() {
        let (env, client, admin, _user) = setup();
        let result = client.try_initialize(&admin);
        assert!(result.is_err());
    }
}